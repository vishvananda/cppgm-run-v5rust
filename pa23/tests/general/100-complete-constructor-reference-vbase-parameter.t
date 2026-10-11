struct B
{
  int value;
  B() : value(7) {}
  B &operator=(B const &other)
  {
    value = other.value;
    return *this;
  }
};

struct H : virtual B {};
struct E : virtual H {};

struct W : E
{
  W(E const &e) : E(e)
  {
    copy_from(&e);
  }

  void copy_from(void const *) {}
  void copy_from(B const *p)
  {
    static_cast<B &>(*this) = *p;
  }
};

int main()
{
  E e;
  B &b = e;
  b.value = 9;
  W w(e);
  B &wb = w;
  return wb.value == 9 ? 0 : wb.value;
}
