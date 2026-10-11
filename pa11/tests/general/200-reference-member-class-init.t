struct Box {
  int x;
};

struct Holder {
  Box& ref;

  Holder(Box& box) : ref(box) {}
};

int main() {
  Box box;
  box.x = 7;
  Holder holder = Holder(box);
  holder.ref.x = 9;
  return box.x;
}
