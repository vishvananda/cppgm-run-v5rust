template<class T> struct Concrete {};
template<template<class> class TT = Concrete> struct Box {};
