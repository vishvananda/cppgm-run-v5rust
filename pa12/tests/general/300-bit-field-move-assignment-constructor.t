// A constructor initializes bit-fields in declaration order regardless of
// whether the class's implicit move assignment is used. A move transfers
// each field's value, including fields that share an allocation unit.
struct Bits {
  unsigned first : 1;
  unsigned second : 2;

  Bits() : first(1), second(2) {}
};

int main() {
  Bits source;
  Bits target;
  source.first = 0;
  source.second = 3;
  target = static_cast<Bits&&>(source);
  return target.first != 0 || target.second != 3;
}
