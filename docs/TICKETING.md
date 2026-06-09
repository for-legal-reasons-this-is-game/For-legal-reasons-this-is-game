## Bug Ticket Guidelines ##
Tickets should be made via a GitHub Issue on the repo. Usually, the person responsible for that part of the code should take care of the ticket.
Tickets should be as descriptive as possible and provide clear information on how to reproduce. Collect any logs or relevant information, and tag the responsible person in the ticket itself (if it can be determined who is responsible, otherwise, tag me instead)
Use this template:
~~~
Details:
Steps to Reproduce:
Affected element:
Responsible Contributor:
~~~

on every weekly meeting, we will go over all tickets, with each of us informing the rest of the status and details of the ticket. Additionally, any  tickets we can’t determine who’s responsible for will be discussed and assigned at the meeting.

## Feature tickets ##
If the ticket is to request a specific feature, please use this template:
~~~
Details:
Expected Outcome: 
Affected Element:
Responsible contributor:
~~~
the rest of the instructions remain the same as with the bug tickets.

## Naming Convention ##
Please follow this naming convention for the title of the GitHub issue:
**(TYPE):(TICKET CODE): (DESCRIPTIVE TITLE)**

The type can either be “BUG” for bugs, or “FEAT” for features. Always use one of these two. If in doubt, ask me. 

The ticket code is comprised of a set of characters indicating the component, a “-“ character, and a sequential number. 

## Ticket classifiers ##
Please, only use these classifiers; if we need more contact me first, simply to avoid having random ticket classifiers:
- DB -> database
- BE -> back end
- TE -> trading engine
- FE -> Front end
- UK -> unknown

Only use UK if it’s relevant; id rather you ask me for a new classifier than using that a lot.

## Numbering ##
The numbering is sequential and corresponds with the type + the classifier’s ticket number. So, we could have ticket *BUG:BE-1: Thing doesn’t work* and *FEAT:BE-13: Add rusty stuff* at the same time, as BUG:BE tickets are counted differently from FEAT:BE tickets. 
Of course, each classifier also has its own independent counting too. Check the other tickets so the sequential counting for each individual type + classifier is correct.

## Guidelines for the comments on tickets ##
Every fix, discussion or communication should be reflected in the ticket too. It’s not enough to talk about it IRL. If you do, add a comment. Everyone should be able to reasonably know the status of a ticket just by going to that ticket and reading the comments.
