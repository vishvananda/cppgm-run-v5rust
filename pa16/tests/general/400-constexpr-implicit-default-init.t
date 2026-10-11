struct Box {
  int x;
  constexpr Box() : x(0) {}
};

constexpr Box box;
static_assert(box.x == 0, "");

int main() { return 0; }
