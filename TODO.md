
* git remote url (git remote -v) returns, at least, these formats:
  - git@github.com:openpubmobus/mobdtimer.git
  - git@github.com:/openpubmobus/mobdtimer.git
  - https://github.com:/openpubmobus/mobdtimer.git
  - https://github.com/openpubmobus/mobdtimer.git

* put in PR to remotemobprogramming/mob that returns 0 if mobbing, 255 if not?, 1 if not in git repo
    alternate: make the expected string a config value

* Split out string functions from main.rs into separate module