// VALIDATION: compile-pass
// N3485 focus: 7.2 [dcl.enum]
// Fixed unsigned enum bases preserve unsigned loads and comparisons in LowIR.

enum class kind : unsigned char {
  value = 133
};

kind input = kind::value;

int main() {
  return input == kind::value ? 0 : 1;
}
