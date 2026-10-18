// Allocated byte objects follow the ordinary Place typed-access exclusion.
// N3485 5.3.4p15: new-initializers initialize the allocated object. expect exit 0
int main() {
  bool* b = new bool(true);
  char* c = new char(3);
  unsigned char* u = new unsigned char(4);
  volatile char* v = new volatile char(5);
  int ok = *b && *c == 3 && *u == 4 && *v == 5;
  delete b;
  delete c;
  delete u;
  delete v;
  return ok ? 0 : 1;
}
