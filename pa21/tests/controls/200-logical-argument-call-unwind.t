// expect exit 0: a call after a logical-argument join still owns live cleanups.
int live;
struct G { G() { ++live; } ~G() { --live; } };
void fail(bool) { throw 7; }
void run() { G g; fail(live == 1 && live != 0); }
int main() {
  try { run(); }
  catch (int value) { return value != 7 || live != 0; }
  return 1;
}
