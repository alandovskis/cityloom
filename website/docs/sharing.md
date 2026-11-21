---
id: sharing
title: Sharing a design
sidebar_position: 3
---

# Sharing a design

A share link lets someone else see one of your projects as it is now, without an account and without being able to
change it. This page says how to make and open one, and what CityLoom keeps, for whom and for how long.

## Making a link

On the home page, choose **Share** on a project's row. The share page (`share.html?project=<id>`) explains what a link
shows; choose **Create a share link**.

- You must be signed in: the server keeps the shared copy for your account. Signed out, the page says so and makes no
  link.
- The link is shown in a field you can select, with **Copy link** and the day it stops working. The links you made are
  listed there again on the same device, newest first.
- **Delete link** deletes the shared copy from the server before it expires. Anyone who opens the link afterwards is told
  the person who shared it deleted it.
- An account holds at most 50 links that still work, makes at most 100 links in any 24 hours (deleted ones count
  too), and a shared copy is at most 1 MB.

## Opening a link

A link looks like `https://…/share.html#<token>`. Opening it needs no account: the page fetches the shared copy, checks
it, keeps it in this browser as a copy of its own, and opens its map read-only (`view=1`), as any design opened
read-only: everything can be looked at, nothing can be changed. The copy is not listed among your projects, is never
sent to the server with your work, and is replaced if you open the same link again. A browser keeps at most ten such
copies, the oldest going first.

To change a shared design, choose **Edit a copy** in the top bar, beside **Read-only**. It needs no account: the
design becomes a new project of your own, with the same name, area, edits and area rules, listed on the home page and
synced with your work like any other project, and the page offers **Open your copy**. Your project is independent:
what you change in it never reaches the shared design, its creator or anyone else who opens the link, and opening the
link again replaces the read-only copy, not your project.

When a link cannot be opened, the page says why and keeps nothing: the link is not whole, no shared design has it, it
has expired, its creator deleted it, it was made by a newer CityLoom, it is damaged, the network or the server cannot be
reached, or the browser's storage refused the copy.

## Privacy and retention

- **What is shared**: a snapshot of the whole project when the link was made: its name, its area, its street and
  junction edits, and its area rules. Not its timeline, nor anything of your account, your settings or your other
  projects. Changes you make to the project afterwards are not in the link: make a new one to share them.
- **Who can see it**: anyone who has the link, and nobody else. The link's secret (256 random bits) is in the part of
  the address after `#`, which browsers never send to a server as part of an address, nor to another site in a
  `Referer`. To fetch the design, the share page sends the secret to CityLoom's server alone, in a request header
  (`Share-Token`) rather than in an address, which proxies along the way may log; the server uses it to find and
  decrypt the snapshot, and does not keep it. Treat a link like a key: whoever you give it to can pass it on.
- **What the server keeps**: the snapshot, encrypted under a key derived from the link's secret, so that the server
  holds nothing it could read without the link; a fingerprint of the secret to find it; your account, as its creator;
  and when it was made and expires. The secret itself is never stored, nor written in the server's logs, which record
  no request header.
- **For how long**: 90 days from when the link was made. The snapshot is then deleted, and the link says it has expired.
  A deleted or expired link keeps saying so for a year; after that it says no shared design has it.
- **Deleting**: you can delete a link at any time from the share page of the device you made it on. Deleting a
  project on the home page deletes the links made of it on that device first (you must be signed in with the account
  that made them); if one cannot be deleted, the page says why and offers to delete the project anyway, its links then working, and
  counting toward the 50 links an account may hold, until they expire: they are no longer listed anywhere. A project deleted on another
  device leaves the links made of it on this one working until they expire. Deleting your
  account deletes every link you made at once; those links then say no shared design has them.
- **On the opener's device**: the copy is kept in that browser's storage only, until it is replaced or the ten-copy
  limit pushes it out, or the browser's site data is cleared.
