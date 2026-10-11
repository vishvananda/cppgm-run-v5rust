// VALIDATION: compile-pass
// N3485 focus: 13.5 [over.oper], 14.5.2 [temp.mem]

struct payload
{
  int value;
};

template<class D>
struct sink
{
  int total;

  sink & operator<<(const char *)
  {
    total = -100;
    return *this;
  }

  template<class T>
  sink & operator<<(const T & value)
  {
    total = total + value.value;
    return *this;
  }
};

int main()
{
  sink<int> out;
  payload value;
  out.total = 0;
  value.value = 7;
  sink<int> * selected = &(out << value);
  return selected == &out && out.total == 7 ? 0 : 1;
}
