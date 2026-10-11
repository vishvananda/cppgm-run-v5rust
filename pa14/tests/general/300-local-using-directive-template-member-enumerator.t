namespace flags {
enum syntax_option_type {
  ECMAScript = 1,
  basic = 2
};
}

template<class T>
struct Compiler {
  typedef flags::syntax_option_type Flag;

  static Flag validate(Flag f)
  {
    using namespace flags;
    switch (f & (ECMAScript | basic)) {
    case ECMAScript:
    case basic:
      return f;
    default:
      return ECMAScript;
    }
  }
};

int main()
{
  return Compiler<int>::validate(flags::ECMAScript) == flags::ECMAScript ? 0 : 1;
}
