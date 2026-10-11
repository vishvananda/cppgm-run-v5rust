// VALIDATION: compile-pass
// A concrete template-id under const& is more specialized than a generic T&,
// including when the template-id binds a template-template parameter.

template<class T>
struct actor {};

template<class T, template<class> class Actor>
struct keyword {};

template<class T, class Domain>
struct protoify;

template<class T, class Domain>
struct protoify<T&, Domain>
{
  typedef char type;
};

template<class Descriptor, template<class> class Actor, class Domain>
struct protoify<keyword<Descriptor, Actor> const&, Domain>
{
  typedef long type;
};

static_assert(
    sizeof(protoify<keyword<int, actor> const&, void>::type) == sizeof(long),
    "specific const-reference partial specialization");

int main()
{
  return 0;
}
