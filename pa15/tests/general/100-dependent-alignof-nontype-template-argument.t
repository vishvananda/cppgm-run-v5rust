typedef unsigned long size_type;

template<class T, T Value>
struct constant
{
  static const T value = Value;
};

template<class T>
struct alignment_of : constant<size_type, alignof(T)>
{
};

struct Payload
{
  bool value;
};

int main()
{
  return alignment_of<Payload>::value == alignof(Payload) ? 0 : 1;
}
