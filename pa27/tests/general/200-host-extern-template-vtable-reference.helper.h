struct Root
{
};

template<class T>
struct Facet : virtual Root
{
  T value;
  Facet(T input) : value(input) {}
  virtual int read() const;
  virtual ~Facet();
};

template<class T>
int Facet<T>::read() const { return value + 2; }

template<class T>
Facet<T>::~Facet() {}

extern template struct Facet<int>;
int host_read(const Facet<int> *);
