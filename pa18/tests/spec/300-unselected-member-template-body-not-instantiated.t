// N3485 focus: 13.3 [over.match], 14.7.1 [temp.inst]
// Selecting one member-template overload does not instantiate the incompatible
// body of a sibling member-template overload.

struct true_type
{
};

struct false_type
{
};

struct iterator
{
};

struct range
{
  void fill(unsigned long, const int&);

  template<class T>
  void initialize(T count, T value, true_type)
  {
    fill(static_cast<unsigned long>(count), value);
  }

  template<class T>
  void initialize(T, T, false_type)
  {
  }

  template<class T>
  range(T first, T last)
  {
    initialize(first, last, false_type());
  }
};

int main()
{
  iterator first;
  iterator last;
  range value(first, last);
  (void)value;
  return 0;
}
