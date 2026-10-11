namespace boost_like {
namespace json {

struct key_value_pair
{
  long value;
};

struct object
{
  using index_t = int;

  struct table
  {
    static inline table * allocate();
  };
};

inline object::table * object::table::allocate()
{
  typedef index_t found_index_t;
  return 0;
}

} // namespace json
} // namespace boost_like
