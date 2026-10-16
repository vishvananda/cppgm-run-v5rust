// expect reject
// A required default diagnoses the declarator that forms an invalid pointer.
template<class T, class U = T*> struct Box;
using Invalid = Box<int&>;
int main() {}
