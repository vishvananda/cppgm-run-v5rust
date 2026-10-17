// N3485 3.2, 9.4.2: constexpr pointer reads use evaluated addresses while
// address-of still designates the member's storage; expect exit 0
int effects;
int data[3];
struct Constants {
  static constexpr int* pointer = data + 1;
  static constexpr const char* text = "value";
};
constexpr int* Constants::pointer;
constexpr const char* Constants::text;
Constants object;
Constants& get() { ++effects; return object; }
int main() {
  int* const* slot = &Constants::pointer;
  int* pointer = get().pointer;
  const char* const* text_slot = &Constants::text;
  return *slot == pointer && pointer == data + 1 && *text_slot == Constants::text
      && Constants::text[0] == 'v' && effects == 1 ? 0 : 1;
}
