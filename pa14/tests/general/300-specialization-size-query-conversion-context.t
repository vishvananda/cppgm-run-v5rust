// Size and alignment query provenance survives aliases, cv and parentheses.
template<class T> struct cell { T value; };
typedef cell<long> alias;
typedef alias &reference;
typedef alias array[2];

long sizes(alias &object) {
  return sizeof(const alias) + 2
      + (sizeof(reference) + 3)
      + (sizeof(array) + 4)
      + (sizeof(object) + 5)
      + (alignof(const alias) + 6)
      + (alignof(reference) + 7)
      + (alignof(array) + 8)
      + (alignof(object) + 9);
}

int main() {
  alias object;
  return sizes(object) == 116 ? 0 : 1;
}
