struct interval {};

struct provider {
  using shorter_interval_type = interval;
};

template<class ReturnType, class IntervalType>
ReturnType compute(int) {
  return ReturnType();
}

int test(provider interval_type_provider) {
  return compute<int, typename decltype(interval_type_provider)::shorter_interval_type>(0);
}

int main() {
  provider p;
  return test(p);
}
