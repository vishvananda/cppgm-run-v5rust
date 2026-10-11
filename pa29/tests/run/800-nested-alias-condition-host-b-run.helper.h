// Declarations the units share.
#pragma once
#include <cstdio>
template<bool B, class T = void>
struct enable_if
{};
template<class T>
struct enable_if<true, T>
{
	typedef T type;
};
int from_b();
