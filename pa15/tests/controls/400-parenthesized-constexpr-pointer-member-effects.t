// N3485 9.4p2, 3.2: parenthesized constant reads evaluate the receiver; expect exit 0
int effects;
int data[3];
struct Constants {
  static constexpr int* pointer = data + 1;
  static constexpr int* null = nullptr;
};
Constants object;
Constants& get() { ++effects; return object; }
int main() {
  int* first = (get().pointer);
  int* second = ((get().null));
  int* third = (((get().pointer)));
  return first == data + 1 && second == nullptr && third == first
      && effects == 3 ? 0 : 1;
}
