struct Root { virtual ~Root() {} };
struct Side { virtual ~Side() {} };
template<class T> struct Common : Root { virtual ~Common(); };
template<class T> struct Layer : Common<T> { virtual ~Layer(); };
template<class T> struct Route : Side, T {};
struct Leaf : Route<Layer<Leaf> > {};
extern template class Common<Leaf>;
extern template class Layer<Leaf>;
Root *make_object();
