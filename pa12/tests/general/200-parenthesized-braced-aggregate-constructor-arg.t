// VALIDATION: compile-pass

struct Payload
{
  int code;
  int detail;
};

struct Holder
{
  Payload value;

  Holder(const Payload & v) : value(v) {}
  Holder(Payload && v) : value(static_cast<Payload &&>(v)) {}
};

int main()
{
  Holder direct({1, 2});
  Holder functional = Holder({3, 4});
  return direct.value.detail + functional.value.code - 5;
}
