// VALIDATION: compile-pass
// Function-template deduction ignores an alias template's retained parameter spelling.
template<bool B, class T, class U = int> struct message;
template<bool B, class T, class U> struct message {};
template<class T, class U = int> using response = message<false, T, U>;
template<bool B, class T> void read(message<B, T, int>&) {}
int main() { response<char> value; read(value); }
