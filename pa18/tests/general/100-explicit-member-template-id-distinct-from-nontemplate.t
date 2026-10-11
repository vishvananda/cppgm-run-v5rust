// An explicit member-template-id must retain template identity through output
// selection even when a non-template overload has the same function type.
struct stack_like {
  bool pop(long& value) {
    return pop<long>(value);
  }

  template<class U, class Enabler = void>
  bool pop(U&) {
    return true;
  }
};

int main() {
  stack_like value;
  long result = 0;
  return value.pop(result) ? 0 : 1;
}
