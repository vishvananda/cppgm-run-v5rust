struct B { int x; };
struct D : B {};
struct Box {
  D value;
  D & operator*() { return value; }
};

B & f(Box & box) { return *box; }

int main() {
  Box box;
  box.value.x = 1;
  B & ref = f(box);
  ref.x = 7;
  return box.value.x == 7 ? 0 : 1;
}
