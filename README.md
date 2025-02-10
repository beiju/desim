Desim
=====

**Desim** is an effort to match RNG outputs to their uses in Blaseball's Beta
era using game update data. This is its main difference from the much more 
mature [resim][resim], which does the same but from Feed event data. Working 
with game update data means we can theoretically solve for the Discipline Era 
(hence the "de" in "desim"), but it also means that we have to figure out game
tick ordering, something that resim gets for free from the Feed.

[resim]: https://github.com/xSke/resim

Desim is in very early development. Currently it's just trying to match Resim's 
output when only a single game is running, and therefore when we don't have to
worry about game tick ordering.

Running
-------

To run Desim (short version for experts):
1. Install Rust
2. From the `desim` directory, run `cargo run`

To run Desim (verbose version):
1. Install Rust using the instructions at https://rustup.rs/
2. If you haven't already done so, `checkout` this repo with Git
3. At the command line, navigate to the `desim` directory within this repo's 
   main directory and run `cargo run`
   1. Note: Don't confuse the `desim` directory *within* this repo with the 
      repo's own directory, which by default is also called `desim`. If you 
      didn't change the name at checkout, you'll run from `desim/desim`.
   2. Both Cargo (at build time) and Rocket (at runtime) need the directory to 
      be correct. If you separate the build and run steps, make sure you also 
      run from the correct directory.  
   3. If the default port is in use on your machine, you can use the 
      `ROCKET_PORT` environment variable to choose a different port. Different 
      shells have different ways of specifying environment variables. For 
      example, in Bash to use port 5112 you would run 
      `ROCKET_PORT=5112 cargo run`.
4. Visit the URL that gets printed to the console (by default, 
   http://127.0.0.1:4110)
5. Click the entry for the fragment you want to see. As of this writing there 
   is only one listed, so click that.
   1. Theoretically a fragment may have multiple games, but as of this writing
      fragments are limited to showing only the first game within them.

Contributing (Front-end)
------------------------

A living list of front-end issues that are good for first-time contributors is 
[here][front-end-good-first-issues]. Remove the `good first issue` tag from the
search for more advanced issues.

[front-end-good-first-issues]: https://github.com/beiju/desim/issues?q=state%3Aopen%20label%3A%22front-end%22%20label%3A%22good%20first%20issue%22

Desim uses [Tera][tera] as its templating language and endeavors to use vanilla
HTML and CSS as much as possible. This is not to say that contributions 
involving Javascript will be rejected, but big frameworks like React likely 
will be. We're rendering potentially very large documents (millions of table 
rows) which don't change at all after rendering, so a dynamic framework is 
probably not a good choice.

In Tera you can see the data your template receives using the special 
`{{ __tera_context }}` variable. This is already rendered at the bottom of the
`fragments` template for convenience. The structure of this data is determined 
by the backend code.

There is no auto-reload. Most of the time you can just refresh your browser and
see your changes; however, if you've made a syntax error in a template file it
won't update on refresh. If you restart the server while there's a syntax 
error in a template file the server will error out and tell you where your 
syntax error was.

Significant locations are:
- The `desim/static` folder is served under `/static`. You can put any static 
  assets there (the CSS file is already there as an example). Don't confuse this
  with `desim/resources`, which is for the server to read as it boots up.
- The `desim/templates` folder contains the Tera templates. `index` is the 
  default page, `fragments` is what you see after clicking one of the fragments 
  on the index page, and `error` is shown when you get an error or visit the
  `/error-test` page.

[tera]: https://keats.github.io/tera/docs/#templates

Contributing (Back-end)
-----------------------

The major concern for the back end currently is getting Resim-compatibility for
days when only a single game is running (currently, S12D113). Resim 
compatibility is indicated by the ✅ and ❌ marks at right side of the rolls
list: we want those to all be ✅. The workflow here is to find the first ❌, 
figure out why it's an ❌, make it a ✅, and repeat.  

The Resim info that's shown when you hover over the ❌ is helpful for this. Here 
is some general advice on what the types of issue mean:

- If the Roll is completely wrong, that means we got out of sync with resim. 
  Normally this is caused by a previous error and the solution is to fix that.
  If there's no previous error, then our RNG stream got desynced from resim's 
  for some other reason, and we need to figure out why.
- If the Roll is only a little wrong, that points to a data handling issue 
  between resim and us. This is an advanced issue, but luckily you can ignore it
  and move on to the next one accepting that the least significant digits of
  following values might mismatch.
- If it says "Expected no threshold" that means we got a threshold, but resim
  didn't. Either we're computing a spurious threshold, in which case it should
  be removed, or resim is missing a genuine threshold, in which case it should 
  be added to resim and the roll streams should be regenerated (roll stream 
  regeneration documentation is issue #7).
- If it says "Expected threshold" with a number in white, that means resim was 
  able to compute a threshold for this roll but we didn't get one. Getting the 
  value of the applicable threshold should be added to `rolls.rs` and, if a new 
  threshold computation needs to be added, that goes in `thresholds.rs`.
- If it says "Expected threshold" with a number that has some digits in red, 
  that means our computed threshold differed by those digits. If it's only a 
  few low digits, that's probably an order of operations issue. If the numbers 
  are kind of close, but differ by more than 1%, there's probably a modifier 
  that's not being computed correctly. If the numbers are completely wrong, we 
  might be using the wrong data entirely.
- If it says "Expected to not know whether the roll passed", that means we're 
  saying we know whether the roll was a pass or failure but resim doesn't know. 
  Either we're wrong or resim is, and whichever is wrong should be fixed. Or, if
  we have info that resim doesn't (which I'm not sure is possible), 
  infrastructure to mark this roll as such needs to be added. 
- If it says "Expected to know that passed=<something>", that means resim knows
  that this roll passed or failed but we don't. We should be able to know from
  game update data whether this roll passed or failed. That information should 
  be encoded or, if we really can't know it from data available in the 
  discipline era, support for indicating such should be added. I really doubt 
  there's anything that fits that category though.
- If it says "Expected passed=(something) but we predicted (something)", then
  we and resim disagree about whether the check was passed. Check that we're 
  both using the same definition of "passed" (resim has authority if we differ).
  If that's not the reason, this should only happen as a result of a previous
  roll and/or threshold error.
- If at any point you get a TEMPLATE ERROR, then the template is written 
  incorrectly and the front end needs to be fixed.

Resim mismatch issues are not tracked in the issue tracker since they're so 
transient. More fundamental backend issues are tracked in [this tag]
[backend-issues] and help is appreciated.

[backend-issues]: https://github.com/beiju/desim/issues?q=state%3Aopen%20label%3A%22back-end%22