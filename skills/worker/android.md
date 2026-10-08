---
extends: worker/android
---
5. (2026-10-05) Build only with `android/build.sh` (throwaway image, Google's SDK tools never on the
   host). AGP 9.4.1 expects build-tools 36.0.0. The Gradle cache is a folder in the user's home.

## Lint (LINT-01, 2026-10-08)

124. checkstyle (sun_checks) and PMD run on every Java file and block the merge. Write classes `final` with a public no-arg constructor, Javadoc with `@param`/`@return` on every method and field, `final` parameters and locals, names of 3 to 17 characters, lines up to 80 characters, named constants instead of numbers, one `return` per method, and at most 10 methods per class (split helpers into small classes). PMD's LawOfDemeter flags `Build.VERSION.SDK_INT` (import `android.os.Build.VERSION`), another class's constants (keep a local constant) and calls on a getter's result (pass the result straight into a helper).
