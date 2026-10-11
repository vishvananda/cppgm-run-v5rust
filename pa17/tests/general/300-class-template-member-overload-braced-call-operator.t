struct key
{
  int value;
};

template<class T>
struct node_value
{
  T value;

  const T &_M_v() const
  {
    return value;
  }
};

struct identity
{
  template<class T>
  T &&operator()(T &&x) const
  {
    return static_cast<T &&>(x);
  }
};

struct hash_key
{
  int operator()(const key &k) const
  {
    return k.value;
  }
};

template<class Key, class Value, class ExtractKey, class Hash>
struct hash_code_base
{
  int _M_hash_code(const Key &k) const
  {
    return Hash{}(k);
  }

  int _M_hash_code(const node_value<Value> &n) const
  {
    return _M_hash_code(ExtractKey{}(n._M_v()));
  }
};

int main()
{
  node_value<key> n = {{61}};
  hash_code_base<key, key, identity, hash_key> h;
  return h._M_hash_code(n) == 61 ? 0 : 1;
}
