// N3485 focus: 12.3.2 [class.conv.fct], 14.7.1 [temp.inst]
// Instantiating a class template does not instantiate the body of an unused
// conversion function, even when that body requires its argument complete.

template<class T>
struct deferred_conversion
{
  explicit operator bool() const
  {
    return sizeof(T) != 0;
  }
};

struct incomplete;

deferred_conversion<incomplete> value;

int main()
{
  return sizeof(value) == 1 ? 0 : 1;
}
