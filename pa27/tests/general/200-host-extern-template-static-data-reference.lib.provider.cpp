template<class T>
struct Facet
{
  static int id;
};

template<class T>
int Facet<T>::id = 7;

template struct Facet<int>;
