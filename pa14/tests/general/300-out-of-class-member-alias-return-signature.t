template<class A, class B>
struct pair
{
  A first;
  B second;
};

struct node_base {};

template<class T>
struct node_traits
{
  typedef node_base * base_ptr;
};

template<class T>
struct tree
{
  typedef node_traits<T> traits_type;
  typedef typename traits_type::base_ptr base_ptr;

  pair<base_ptr, base_ptr> get(const T & value);

  pair<base_ptr, base_ptr> call(const T & value)
  {
    return get(value);
  }
};

template<class T>
pair<typename tree<T>::traits_type::base_ptr,
     typename tree<T>::traits_type::base_ptr>
tree<T>::get(const T &)
{
  return pair<typename tree<T>::traits_type::base_ptr,
              typename tree<T>::traits_type::base_ptr>();
}

int main()
{
  tree<int> t;
  t.call(1);
  return 0;
}
