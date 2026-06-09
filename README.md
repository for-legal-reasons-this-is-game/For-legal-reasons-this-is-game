*This project has been created as part of the 42 curriculum by mchoma, alvcampo, jel-ghna, megardes.*

# For legal reasons this is game

## Description
**For legal reasons this is game** is an **API-first, real-time simulated trading platform** (ft_transcendence) where users can paper-trade instruments such as **stocks, coins, and options**.

The project’s goal is to design and build a system that is:
- **Fast & scalable** (designed to scale horizontally; Kubernetes planned)
- **Real-time** (live updates for price movements / order events)
- **Accessible via a public API** (clients can be written in multiple programming languages)
- **Modular** (separate trading engine, database layer, and API service)

### Key features (current + planned)
- Public API for market data and trading actions (**planned / in progress**)
- Trading engine that recalculates prices based on platform activity (**planned / in progress**)
- Database seeding from external APIs (**planned**, provider TBD)
- Responsive web frontend (**TBD: Angular/Vue/other**)
- Admin / advanced permission ideas similar to prediction-market resolution flows (**idea stage; not implemented yet**)

---

## Instructions

### Prerequisites
You’ll need:
- **Docker** (required)
- **Docker Compose** (required)
- (Planned later) **Kubernetes** for scalable deployment

> Note: exact versions and environment variables will be pinned once the stack stabilizes.

### Run (development)
The repository is intended to be run via Docker. The exact command may change as the repo structure is finalized.

**Expected workflow (subject to change):**
```bash
docker compose up --build
```

To stop:
```bash
docker compose down
```

If your repository later introduces multiple compose files, document the canonical one here, e.g.:
```bash
docker compose -f docker-compose.dev.yml up --build
```

### Environment variables (.env)
If/when required, copy an example file:
```bash
cp .env.example .env
```
> If `.env.example` does not exist yet, environment variables are still being defined.

### Services (planned architecture)
The system is being designed as multiple components (names/ports TBD):
- **API service (Rust)**: exposes REST endpoints (public API)
- **Trading engine (Rust)**: processes orders/trades and recalculates prices
- **Database (Rust, relational)**: custom distributed relational database (schema in progress)
- **Cache layer (TBD)**: Redis/Memcached/other (not decided)

---

## Team Information
All team members are currently **Developers** (responsibilities will be refined as the project progresses).

- **mchoma** 
    - Developer   
    - Product Owner
    - Technical Lead Architect
  Responsibilities: TBD (initial architecture, requirements, planning input)
- **alvcampo** 
    - Project Manager Scrum Master
    - Developer  
  Responsibilities: TBD
- **jel-ghna** — Developer  
  Responsibilities: TBD
- **megardes** — Developer  
  Responsibilities: TBD

---

## Project Management

### Organization
We organize work using:
- Breaking features into small deliverables (backend, engine, DB, frontend, DevOps)
- Iterative implementation (define → build → test → improve)
- Decisions documented in Issues (so new contributors can catch up)

### Tools
- **GitHub Issues**: task tracking, assignment, progress, technical discussion
- **Discord**: communication, quick syncs, decisions

---

## Technical Stack (current decisions + TBD)

### Frontend
- **TBD** (candidates discussed: Angular / Vue)
- Target: responsive UI, good maintainability for a complex system

### Backend / API
- **Rust**
- Framework: **TBD** (Axum / Actix / Rocket are candidates)
- **REST API** planned (OpenAPI/Swagger planned once endpoints stabilize)

### Trading Engine
- **Rust**
- Separate component/service responsible for price recalculation and order processing (**in progress**)

### Database
- **Custom distributed relational database in Rust**
- Schema: **in progress / not finalized**

### Cache
- **TBD** (Redis/Memcached/other)

### DevOps / Deployment
- **Dockerfiles** for services
- **Docker Compose** for local development
- **Kubernetes** planned later for scalability/redundancy

---

## Database Schema (TBD)
A relational schema is being designed.

Planned deliverable for this section:
- Tables/entities (users, markets, instruments, orders, trades, balances, etc.)
- Relationships (1:N, N:M)
- Key fields and data types
- Diagram (image or ASCII ERD)

> This section is intentionally left incomplete until the schema is finalized.

---

## Features List (living checklist)
This section is intended to help contributors quickly see **what exists** and **what still needs to be built**.

### Implemented
- (none yet / TBD)

### In progress
- Rust backend/API scaffolding (framework TBD)
- Trading engine design and separation into its own component
- Relational DB structure design

### Planned
- Public REST API + OpenAPI/Swagger spec
- Seed database from external market data APIs (provider TBD)
- Real-time updates (WebSockets or similar)
- Authentication + user management (module)
- Advanced permission system (idea stage)
- Admin-style market resolution tools (idea stage)
- Analytics dashboard with data visualization
- Notifications
- 2FA
- Advanced search
- Data export/import
- Multi-language support (optional)

> Ownership per feature will be tracked in GitHub Issues and filled in once tasks are assigned.

---

## Modules (ft_transcendence)
> Exact final selection and point totals will be validated against the subject once implementation is locked in.

### Major modules (planned)
- Use a framework for both frontend and backend (**frontend framework TBD**, backend framework TBD)
- Implement real-time features using WebSockets (or similar technology)
- Public API
- Advanced permission system
- Standard user management
- Advanced analytics dashboard with data visualization

### Optional / ideas (not committed yet)
- Backend as microservices
- AI module(s) of choice
- “Gamification” concepts (not a priority)

### Minor modules (planned)
- Notification system
- 2FA
- Analytics dashboard (if required separately from major scope)
- Advanced search functionality
- Data export/import
- Support for 3 languages (optional)

---

## Individual Contributions (TBD)
This section will be completed as the project progresses.

Planned format:
- **mchoma**: features/modules/components + challenges + solutions
- **alvcampo**: features/modules/components + challenges + solutions
- **jel-ghna**: features/modules/components + challenges + solutions
- **megardes**: features/modules/components + challenges + solutions

---

## Resources

### Classic references
- Rust Book (official): https://doc.rust-lang.org/book/
- Rust API Guidelines: https://rust-lang.github.io/api-guidelines/
- OpenAPI Specification: https://spec.openapis.org/oas/latest.html
- Swagger (OpenAPI tools): https://swagger.io/specification/
- WebSockets (MDN): https://developer.mozilla.org/en-US/docs/Web/API/WebSockets_API
- REST API design (Microsoft guidelines): https://github.com/microsoft/api-guidelines
- System design / scalability basics:
  - https://12factor.net/
  - https://kubernetes.io/docs/concepts/
  - https://docs.docker.com/compose/

### How AI was used
AI tools were used for:
- Turning meeting notes into a structured README/checklist
- Drafting documentation templates and contributor-facing checklists
- Brainstorming architecture options and tradeoffs (frameworks, messaging, caching)

> AI-generated suggestions are reviewed by the team before being adopted.

---

## Notes / Known limitations (current)
- Frontend framework is not selected yet.
- Backend Rust framework is not selected yet.
- Cache solution is not selected yet.
- Database schema is in progress.
- External API provider for seeding is not selected yet.
