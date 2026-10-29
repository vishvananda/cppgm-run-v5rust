// A typed reference handler needs RTTI base offsets without an object instance.
struct Base { int value; };
struct Derived : Base { int other; };
int main() {
  try { throw 0; }
  catch (const Derived&) { return 1; }
  catch (int value) { return value; }
}
