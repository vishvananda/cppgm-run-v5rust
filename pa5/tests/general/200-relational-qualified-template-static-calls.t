namespace limits_support {
typedef long integer;

template<class T>
struct limits {
  static T min();
  static T max();
};
}

struct address {
  long offset;
};

bool outside(long distance, const address& value) {
  return distance < limits_support::limits<limits_support::integer>::min() ||
         value.offset > limits_support::limits<limits_support::integer>::max();
}
