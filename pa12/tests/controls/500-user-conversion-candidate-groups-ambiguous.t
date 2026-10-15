// expect reject
// Constructor and conversion-function groups compete before deletion/access checks.
struct source;
struct target { target(source&); };
struct source { operator target() = delete; };
void consume(target);
void test(source& value) { consume(value); }
