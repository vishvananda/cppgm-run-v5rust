namespace ns {
template<class T> struct traits {};
template<class T, class Traits> struct stream {};

template<class T, class Traits>
stream<T, Traits>& passthrough(stream<T, Traits>& value)
{
  return value;
}
}

typedef ns::stream<char, ns::traits<char> > stream_type;
typedef stream_type& (*passthrough_type)(stream_type&);

stream_type global_stream;
passthrough_type global_passthrough =
    &ns::passthrough<char, ns::traits<char> >;

int main()
{
  return global_passthrough ? 0 : 1;
}
