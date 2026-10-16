// expect exit 0
// The frozen compiler applies only a root declarator's expansion marker.
template<class T> void select(T (*... value)(T));
template<class T> int variadic(T, ...);
int main() { return 0; }
