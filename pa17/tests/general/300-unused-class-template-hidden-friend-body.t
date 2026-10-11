template<class T>
bool less_value(T a, T b)
{
  return a < b;
}

template<class T>
struct vec
{
  friend bool operator<(const vec &, const vec &)
  {
    T a;
    return less_value(a, a);
  }
};

struct X
{
};

vec<X> global_vec;

int main()
{
  return 0;
}
