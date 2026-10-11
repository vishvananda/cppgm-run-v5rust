// A function return type does not require its class-template argument complete.
class object;
template<class T> struct pair { T first; };
pair<object> generate();
class object {};
pair<object> instance;
