---
id: projects
title: Projects
sidebar_position: 2
---

# Projects

Your work in CityLoom is kept in projects. A project has a name and an area: its map shows that area's streets and
junctions, and its street edits, junction edits and area rules belong to it alone. Two projects can change the same
street in two different ways.

## The list of projects

The home page (`web/index.html`) lists your projects under **Your projects**, the one changed most recently first. Each
row shows the project's name, its area and the day it was last changed.

- **New project**: type a name under **Project name** and choose **Create project**. A name has 1 to 80 characters. A
  new project's area is always Plateau Mont-Royal, Montréal: it cannot be chosen yet. Once a project is made, its area
  does not change.
- **Open**: choose the project's name. Its map opens (`map.html?project=<id>`), and the streets and junctions you open
  from there are edited in that project.
- **Rename**: choose **Rename**, type the new name and choose **Save the name**. Escape or **Cancel** leaves it as it
  was.
- **Delete**: choose **Delete**. The row asks first; **Delete the project** removes the project and everything in it.
  This cannot be undone.

The map and the editors carry the project in their address. A page opened without one opens the project you opened
last. If that project is no longer there, for example because it was deleted on another device, the page goes back to
the list, which says **That project is no longer here.**

## Work from before projects

Work kept in the browser by a CityLoom that had no projects moves, the first time the list opens, into a project named
**First project**, in the same area as before if that area was in the Montréal region, else in Plateau Mont-Royal,
Montréal. It opens as it did, so nothing has to be done.

## Projects on several devices

When you are signed in and chose **Store my work on the server** on the Account page, your projects travel with your
work: each device you sign in on receives every project, each with its own edits and rules. A project created, renamed
or deleted on one device is created, renamed or deleted on the others the next time they sync. When you choose server
storage, the message after the merge counts places (streets, junctions, area rules), not the projects themselves.

Work an older CityLoom stored on the server belongs to the first project.

## The design archive

**Export design** on the Account page saves the work of the project you opened last. **Import design** adds the work of
an archive to that project.
