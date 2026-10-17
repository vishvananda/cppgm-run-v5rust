// expect reject
// Forming the argument's declared target diagnoses its absent member type.
struct tag {};
template<class T, typename T::type Value> struct argument {};
argument<tag, 0> value;
int main() { return 0; }
