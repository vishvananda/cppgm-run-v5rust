// VALIDATION: compile-pass
// N3485 focus: 12.2 [class.temporary], 12.4 [class.dtor], 14.5.1 [temp.class]

template<bool B>
struct integral_constant {
  static const bool value = B;
};

template<class T>
struct trait : integral_constant<true> {
};

struct Payload {
  int value;
};

int consume(integral_constant<true>)
{
  return 0;
}

int main()
{
  return consume(trait<Payload *>());
}
