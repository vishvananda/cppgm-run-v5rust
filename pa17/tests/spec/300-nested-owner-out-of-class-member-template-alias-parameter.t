// VALIDATION: compile-pass
// N3485 focus: 14.5.2 [temp.mem]

namespace protocol {

template<class Allocator>
struct fields
{
};

template<bool IsRequest, class Body, class Fields>
struct message
{
};

template<class Body, class Fields = fields<int> >
using request = message<true, Body, Fields>;

} // namespace protocol

template<class Layer, bool Enabled>
struct stream
{
  struct implementation;
};

template<class Layer, bool Enabled>
struct stream<Layer, Enabled>::implementation
{
  template<class Body, class Allocator>
  int build(protocol::request<Body, protocol::fields<Allocator> > const &);
};

template<class Layer, bool Enabled>
template<class Body, class Allocator>
int stream<Layer, Enabled>::implementation::build(
    protocol::request<Body, protocol::fields<Allocator> > const &)
{
  return sizeof(Body) + sizeof(Allocator);
}

int main()
{
  stream<int, true>::implementation value;
  protocol::request<int, protocol::fields<char> > request;
  return value.build<int, char>(request) ==
             static_cast<int>(sizeof(int) + sizeof(char)) ?
      0 : 1;
}
