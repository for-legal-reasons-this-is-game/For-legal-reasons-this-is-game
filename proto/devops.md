# Kubernetes requirements for the trading contract

This file lists what the cluster has to provide so that the design in
`how_to_use.md` holds. It doesn't contain manifests. Each section says what a
component needs and why.

## Components and how they run
**ATTENTION: STATEFUL SET FOR ENGINE IS TBD. TREAT ALL OF ITS MENTIONS AS TBD**
| Component | Kind of workload | Replicas |
|---|---|---|
| Engine | `StatefulSet` with a headless Service | One pod per instance. Each pod owns up to 8 markets. |
| Router | `Deployment` | One or more. It keeps no state of its own. |
| Event consumer | `Deployment` with the `Recreate` strategy | Exactly 1. |
| Backend | `Deployment` | As needed. It only talks to the router. |
| Redis | Highly available setup (managed service or Sentinel/cluster) | Not a single pod. |

## Engine

### Identity

- A pod's name (`engine-0`, `engine-1`, ...) is the instance's identity. The
  engine reads it from the downward API and uses it as the prefix of its keys in
  Redis. After a restart, the pod comes back with the same name and finds its
  own state.
- A `Deployment` can't be used: its pod names change on every restart, so a
  restarted pod would not find its old state.

### At most one process per instance

Two processes writing the same Redis keys corrupt the state.
 
- A `StatefulSet` never runs two pods with the same name at the same time.
  Keep it that way.
- If a node becomes unreachable, the pod is not replaced automatically, and the
  markets on it stay down until the node recovers or an operator removes the
  pod. Don't force-delete a pod unless you are sure its node is dead. A
  force-delete of a pod that is still running creates a second writer.
- Don't run a second copy of an instance for any other reason, such as a
  blue/green deployment or a manual copy.

### Shutdown

- On shutdown, Kubernetes sends `SIGTERM` and then `SIGKILL` after
  `terminationGracePeriodSeconds`. The engine uses that time to answer every
  queued command with `INTERNAL_ERROR` and write its state to Redis. Set the
  period well above the time that takes at the largest expected queue.
- The engine process has to receive the signal. If the container starts it
  through a shell script, run it with `exec`, or make it the container's main
  process, otherwise the shell swallows `SIGTERM`.
- Updates are rolled one pod at a time. Use a conservative update strategy
  (for example `OnDelete`, or a `RollingUpdate` partition) so that you decide
  when each instance restarts. Each restart makes that instance's markets
  unavailable for a short time.

### Hard kills

`SIGKILL` runs no handler. The engine's write-before-send rule covers it, but
the cluster should make it rare:

- Set memory requests equal to limits, with headroom. An out-of-memory kill is a
  `SIGKILL`.
- Alert on `OOMKilled`, on restarts, and on node failures.

### Probes

- **Startup probe:** the engine loads its books, command cache, and event
  history from Redis before it answers commands. Give the probe enough time for
  the largest state, so that Kubernetes doesn't kill the pod while it loads.
- **Readiness probe:** report ready only after the load is complete. Until
  then, the router must not send commands to the pod.
- **Liveness probe:** keep it separate from readiness and lenient. A slow Redis
  must not make Kubernetes restart the engine.

### Network

- A headless Service gives each pod its own DNS name. The router uses those
  names to reach a specific instance.
- A `NetworkPolicy` allows traffic into the engine only from the router.

### Clock

`tb::id()` and UUIDv7 depend on the clock. Run time synchronisation on the nodes
and alert on large clock steps. IDs must not go backwards.

## Redis

The engine assumes Redis is always available, and it sits on the path of every
response, because the engine writes to it before it replies.

- Run it highly available. A failover must not lose acknowledged writes
  silently.
- Enable append-only persistence. Choose the fsync policy on purpose:
  `everysec` can lose about a second of writes after a Redis crash, and
  `always` loses none but is slower. The write-before-send rule is only as
  strong as this setting.
- Eviction policy: `noeviction`. Redis must never evict the engines' keys. Don't
  set expiry on state keys. The engine trims its own event history.
- Size the memory for the books, the command cache, and the event history of
  all instances, at the retention window the engines use.
- Keep latency low and stable: place Redis close to the engines (same zone) and
  alert on latency.
- Limit access to the engines. The router, backend, and consumer don't need
  Redis.

## Router

- The router holds two maps: which market belongs to which instance, and the
  address of each instance. Keep them in a `ConfigMap` or another source the
  router reads at startup and on change. The backend and the consumer never read
  it.
- It is stateless, so it can have several replicas. Each replica opens its own
  stream to every engine instance.
- gRPC uses long-lived HTTP/2 connections. A Layer 4 load balancer in front of
  the router pins all traffic of a client to one replica, and a Layer 7 one
  needs HTTP/2 support. Check how the backend and the consumer reach the router.
- Enable gRPC keepalive between the consumer and the router, and between the
  router and the engines. Make sure no proxy, ingress, or service mesh in the
  path has an idle or maximum-duration timeout that cuts a stream.
- A router replica that restarts drops the consumer's stream. The consumer
  reconnects and replays; that is expected.

## Event consumer

- Run exactly one. Two consumers would process the same events at the same time,
  which the design can survive but doesn't need to.
- Use the `Recreate` strategy, so that the old pod stops before the new one
  starts.
- It keeps no stream position. After a restart it replays the engines' whole
  retained history, so give it enough CPU and memory to do that quickly.
- Its downtime must stay well below the engines' retention window. Alert when it
  has been down for a fraction of the window, and treat it as an incident.
- It writes to TigerBeetle and Postgres. Both must be reachable from it.

## Disruptions

A single pod per instance means that a node drain or a node failure makes its
markets unavailable. A `PodDisruptionBudget` for the engines would block
drains, so it's usually better to drain nodes deliberately, one instance at a
time, with the market status set to halted in the backend first.

## Alerts

- An engine pod restarted, was `OOMKilled`, or isn't ready.
- Redis is unavailable, failed over, or slower than expected.
- The consumer is down, or has been down for a fraction of the retention window.
- A rising number of `INTERNAL_ERROR` responses.
- The consumer stopped on a failed transfer chain.
- Clock steps on the nodes.

## Open points

- The retention window is a value you set when you deploy: long enough to cover
  the longest outage of the consumer you accept, short enough that replaying it
  after a restart is fast and fits in Redis.
- Which Redis setup to use (managed or self-run) and which fsync policy.
- How the router's maps are updated when instances are added.
