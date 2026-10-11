int read(const char* value) { return *value; }

template<class>
struct traits
{
  static constexpr const char* name = "x";
};

int main() { return read(traits<int>::name) == 'x' ? 0 : 1; }
