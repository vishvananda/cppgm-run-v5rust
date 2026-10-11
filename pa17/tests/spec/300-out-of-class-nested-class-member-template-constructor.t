// VALIDATION: compile-pass
// N3485 focus: 14.5.2 [temp.mem]

template<bool IsRequest, class Fields>
class header;

template<class Fields>
class header<true, Fields> {};

template<class Fields>
class header<false, Fields> {};

template<class File>
struct body
{
  class reader;
  class value_type;
};

template<class File>
class body<File>::value_type
{
};

template<class File>
class body<File>::reader
{
public:
  template<bool IsRequest, class Fields>
  explicit reader(header<IsRequest, Fields> &, value_type &);

  int value;
};

template<class File>
template<bool IsRequest, class Fields>
body<File>::reader::reader(header<IsRequest, Fields> &, value_type &)
  : value(sizeof(File) + sizeof(Fields) + (IsRequest ? 1 : 0))
{
}

int main()
{
  header<false, int> source;
  body<long>::value_type value;
  body<long>::reader result(source, value);
  return result.value == sizeof(long) + sizeof(int) ? 0 : 1;
}
