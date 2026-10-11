template<template<typename> class TT, typename T>
struct Holder { TT<T> value; };

template<typename T>
struct Box { T value; };

int main() { Holder<Box, int> holder; holder.value.value = 9; return holder.value.value - 9; }
