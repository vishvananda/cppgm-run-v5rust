// VALIDATION: compile-pass
// N3485 focus: 14.7.2 [temp.explicit]
// An explicit class-instantiation definition must instantiate an out-of-class
// static data member definition even when the class specialization was already
// formed by an earlier use.

template<class T>
struct facet_id
{
  static int id;
};

struct info : facet_id<info>
{
};

typedef char complete_facet_id_before_definition[sizeof(facet_id<info>)];

template<class T>
int facet_id<T>::id;

template struct facet_id<info>;

int main()
{
  return facet_id<info>::id;
}
