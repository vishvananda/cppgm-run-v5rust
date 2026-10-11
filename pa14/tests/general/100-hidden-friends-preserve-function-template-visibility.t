// VALIDATION: compile-pass
// Hidden non-template friends must not hide an ordinary function template that
// shares their name in the enclosing namespace.

namespace lookup
{

template<class T>
void exchange(T& left, T& right)
{
  T temporary(left);
  left = right;
  right = temporary;
}

struct bit_reference
{
  bool *pointer;

  friend void exchange(bit_reference left, bit_reference right)
  {
    bool temporary = *left.pointer;
    *left.pointer = *right.pointer;
    *right.pointer = temporary;
  }

  friend void exchange(bit_reference left, bool& right)
  {
    bool temporary = *left.pointer;
    *left.pointer = right;
    right = temporary;
  }
};

}

int main()
{
  int left = 1;
  int right = 2;
  lookup::exchange(left, right);

  bool first = false;
  bool second = true;
  lookup::bit_reference reference;
  reference.pointer = &first;
  exchange(reference, second);

  return left == 2 && right == 1 && first && !second ? 0 : 1;
}
