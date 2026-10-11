#pragma once

template<class T>
struct odr_holder
{
  explicit odr_holder(T input);
  T value;
};

template<class T>
odr_holder<T>::odr_holder(T input) : value(input)
{
}
