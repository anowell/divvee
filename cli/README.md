## Divvee CLI

See previous work on jem: https://github.com/anowell/jem



### Ideas

```
# Reserve all uppercase one-letter flags for label prefixes: -M milestone -S sprint
dv list

# Create an issue
dv create

# One-letter flags for label prefixes
dv edit <issue> -a assignee -s status

dv comment -m MESSAGE (or editor)

```



Developer:
- create -m title -d description -a (like 'git commit' with EDITOR option for description)
- list (with various filtered views)
- assign (shorthand for edit --assignee USER)
- {start, stop, close} (shorthand for edit --status STATUS)
- comment
- show (with preconfigured views)
- search
- sync (force sync of changes with remote)
- link (adds a smart link, e.g. git commit, URL, etc..)
- view (shows a particular view of issues, like a sprint board) - todo: how do distinguish from list
- estimate: for i in dv list; do dv estimate $i -i; done

Project Manager (PM):

- create (with epic/project options)
- prioritize
- plan (work into active or future sprint)
- schedule a milestone
- label (issues as part of an epic or milestone)
- monitor (shows timeliens, project tracing, risks, etc.)
- generate (reports on team perf, issue status, milestones... specific views with html options)
- assign (to team or individual)
- ref (relate items together as parent/child, dep, or just references)
- bulk-edit by filter or list

Consider TUI version for interactive workflows on some of these
Consider monitor or watch actions for signalling tasks/projects/etc to monitor more closely


Leadership:

- generate or monitor (reports: project, milestones, risks)
- track-work (resource allocation related acctions)
- assess-risk
- align (projects to goals)... just another label
- create (goal) (maybe separate set-goal action)
- view resource allocation
- view decisions
- analyze
- resources



Labels:
- g-goal (aligned with a goal)
- e-epic (part of an epic)
- m-milestone (delivered as part of milestone)
- d-decision (impacted by some pending decision)
- s-sprint (what sprint is this in)
- a-archive (e.g. -a-2023-sprint-alpha)
- c-customer
- r-release
- p-project


we also need label aliases:
// some for convenience
-s-current -> -s-alpha
-s-next -> -s-beta
-s-next+1 -> -s-gamma
-m-next -> -m-nov-11
-r-next

// allow mapping for common typos
-e-mispell -> -e-misspell

