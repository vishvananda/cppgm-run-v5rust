struct Inner { int x; };
struct Outer { Inner field; int y; };

Outer make() {
  Outer o;
  o.field.x = 7;
  o.y = 3;
  return o;
}

int read(Outer o) { return o.field.x; }

int main() { return read(make()) - 7; }
