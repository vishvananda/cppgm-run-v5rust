// expect reject
template<class... Ts> struct Outer {
    template<class U> static int value;
    template<class T> static int value<Ts*>;
};
