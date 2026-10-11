// N3485 8.4.3p4 (only the declaration that is deleted makes a function
// deleted), 14.7.3: a normally defined explicit specialization of a deleted
// function template is callable; expect exit 0
template<class T> int f(T) = delete;
template<> int f<int>(int) { return 3; }
int main() { return f(1) == 3 ? 0 : 1; }
