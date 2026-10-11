// N3485 8.2: w and x declare functions; y and z define objects.
struct C { C(int); };
void ambiguity(int a) {
  C w(int(a));
  C x(int());
  C y((int)a);
  C z = int(a);
}
