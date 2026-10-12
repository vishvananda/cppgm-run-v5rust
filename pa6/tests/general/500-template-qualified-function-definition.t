namespace N { typedef int Word; template<class T> T identity(T); }
template<class U> U N::identity(U value) { Word word; return value; }
