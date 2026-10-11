struct IOS { int value; IOS() : value(0) {} };
struct Stream : virtual IOS { Stream() {} };

struct Holder
{
  int got;

  template<class T>
  Holder(T &s) : got(s.value) {}
};

int take(Holder h)
{
  return h.got;
}

int main()
{
  Stream s;
  s.value = 7;
  return take(s) == 7 ? 0 : 1;
}
