// expect reject
// Check the terminal declaration kind before forming its template arguments.
template<class T> struct Box;
template<class T> using Alias = Box<T>;
template struct Alias<undeclared>;
