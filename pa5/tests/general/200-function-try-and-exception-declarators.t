struct Failure {};
void body();
void f() try {
  body();
  throw Failure();
} catch (const Failure&) {
  throw;
} catch (...) {
}
void (*fp)() throw(int);
void h(void pfa() throw(int));
