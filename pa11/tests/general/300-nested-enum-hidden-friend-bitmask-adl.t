// ADL for a class-member enumeration includes the enclosing class, so a
// hidden bitmask operator declared by that class is a candidate.
struct options {
  enum flags { first = 1, second = 2 };

  friend flags operator|(flags left, flags right)
  {
    return static_cast<flags>(
        static_cast<unsigned>(left) | static_cast<unsigned>(right));
  }
};

int take(options::flags value)
{
  return value == static_cast<options::flags>(3) ? 0 : 1;
}

int main()
{
  return take(options::first | options::second);
}
