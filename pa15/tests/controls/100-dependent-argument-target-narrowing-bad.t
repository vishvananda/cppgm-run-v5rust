// expect reject
// A dependent first argument does not defer conversion to a later known type.
template<class Tag, class T, T Value> struct argument {};
template<class Tag> struct holder {
  argument<Tag, unsigned char, 256> value;
};
int main() { return 0; }
