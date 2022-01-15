#ifndef UNIT_OF_NUMBER_H
#define UNIT_OF_NUMBER_H

#include <stdint.h>
#include <type_traits>
#include <cmath>
#include <cassert>

namespace UnitOfNumber {

	/// <summary>
	/// 基本数字クラス
	/// </summary>
	/// <typeparam name="__ValueType"></typeparam>
	template<class __ValueType>
	class BaseNumeral
	{
	private:
		__ValueType _value;
		inline __ValueType GetValue() const noexcept
		{
			return _value;
		}
		inline void SetValue(__ValueType v) & noexcept
		{
			_value = v;
		}

	public:
		constexpr BaseNumeral(const __ValueType& v) noexcept : _value(v)
		{}
		constexpr BaseNumeral(const __ValueType&& v = 0) noexcept : BaseNumeral(v)
		{}

		inline BaseNumeral& Create(const __ValueType&& init = 0) noexcept
		{
			return *new BaseNumeral(init);
		}
		inline BaseNumeral& Clone() noexcept
		{
			return Create(GetValue());
		}

		template<class T>
		constexpr __ValueType mod(T lhs, T rhs) noexcept
		{
			if constexpr (std::is_integral<T>::value)
			{
				return lhs % rhs;
			}
			else if constexpr(std::is_floating_point<T>::value)
			{
				return std::fmod(lhs, rhs);
			}
		}

		//キャスト演算子(Cast)
		explicit operator __ValueType() const noexcept
		{
			return GetValue();
		}

		//代入演算子(Assignment)
		template<class T> inline __ValueType operator=(T&& rhs) & noexcept
		{
			SetValue(rhs);
			return GetValue();
		}
		template<class T> inline BaseNumeral& operator=(BaseNumeral&& rhs) & noexcept
		{
			SetValue(rhs.GetValue());
			return *this;
		}

		//単項マイナス演算子と単項プラス演算子(Unary Negation/Plus)
		inline __ValueType operator+() const { return  GetValue(); }
		inline __ValueType operator-() const { return -GetValue(); }

		//算術演算子(Arithmetic)
		template<class T> inline __ValueType operator+(T&& rhs) { return GetValue() + rhs; }
		template<class T> inline __ValueType operator-(T&& rhs) { return GetValue() - rhs; }
		template<class T> inline __ValueType operator*(T&& rhs) { return GetValue() * rhs; }
		template<class T> inline __ValueType operator/(T&& rhs) { return GetValue() / rhs; }
		template<class T> inline __ValueType operator%(T&& rhs) { return GetValue() % rhs; }
		template<>        inline __ValueType operator+(BaseNumeral& rhs) { return GetValue() + rhs.GetValue(); }
		template<>        inline __ValueType operator-(BaseNumeral& rhs) { return GetValue() - rhs.GetValue(); }
		template<>        inline __ValueType operator*(BaseNumeral& rhs) { return GetValue() * rhs.GetValue(); }
		template<>        inline __ValueType operator/(BaseNumeral& rhs) { return GetValue() / rhs.GetValue(); }
		template<>        inline __ValueType operator%(BaseNumeral& rhs) { return mod(GetValue(), rhs.GetValue()); }

		//複合代入演算子(Compound Assignment)
		template<class T> inline void operator+=(T&& rhs) { SetValue(GetValue() + rhs); }
		template<class T> inline void operator-=(T&& rhs) { SetValue(GetValue() - rhs); }
		template<class T> inline void operator*=(T&& rhs) { SetValue(GetValue() * rhs); }
		template<class T> inline void operator/=(T&& rhs) { SetValue(GetValue() / rhs); }
		template<class T> inline void operator%=(T&& rhs) { SetValue(GetValue() % rhs); }
		template<>        inline void operator+=(BaseNumeral& rhs) { SetValue(GetValue() + rhs.GetValue()); }
		template<>        inline void operator-=(BaseNumeral& rhs) { SetValue(GetValue() - rhs.GetValue()); }
		template<>        inline void operator*=(BaseNumeral& rhs) { SetValue(GetValue() * rhs.GetValue()); }
		template<>        inline void operator/=(BaseNumeral& rhs) { SetValue(GetValue() / rhs.GetValue()); }
		template<>        inline void operator%=(BaseNumeral& rhs) { SetValue(mod(GetValue(), rhs.GetValue())); }

		//比較演算子(Compare)
		template<class T> inline bool operator==(T&& rhs) const { return GetValue() == rhs; }
		template<class T> inline bool operator!=(T&& rhs) const { return GetValue() != rhs; }
		template<class T> inline bool operator<=(T&& rhs) const { return GetValue() <= rhs; }
		template<class T> inline bool operator< (T&& rhs) const { return GetValue() <  rhs; }
		template<class T> inline bool operator> (T&& rhs) const { return GetValue() >  rhs; }
		template<class T> inline bool operator>=(T&& rhs) const { return GetValue() >= rhs; }
		template<>        inline bool operator==(BaseNumeral& rhs) const { return GetValue() == rhs.GetValue(); }
		template<>        inline bool operator!=(BaseNumeral& rhs) const { return GetValue() != rhs.GetValue(); }
		template<>        inline bool operator<=(BaseNumeral& rhs) const { return GetValue() <= rhs.GetValue(); }
		template<>        inline bool operator< (BaseNumeral& rhs) const { return GetValue() <  rhs.GetValue(); }
		template<>        inline bool operator> (BaseNumeral& rhs) const { return GetValue() >  rhs.GetValue(); }
		template<>        inline bool operator>=(BaseNumeral& rhs) const { return GetValue() >= rhs.GetValue(); }

		//科学算術
		template<class T> inline auto pow(T&& rhs) { return std::pow(GetValue(), rhs); }
		template<>        inline auto pow(BaseNumeral& rhs) { return std::pow(GetValue(), rhs.GetValue()); }
						  inline auto log() { return std::log(GetValue()); }
		template<class T> inline auto log(T&& rhs) { return std::log(GetValue() / rhs); }
		template<>        inline auto log(BaseNumeral& rhs) { return std::log(GetValue() / rhs.log()); }
						  inline auto abs() { return std::abs(GetValue()); }
	};

	/// <summary>
	/// 数字クラス
	/// </summary>
	using NumeralValueType = intmax_t;
	//using NumeralValueType = long double;
	using Numeral = BaseNumeral<NumeralValueType>;

#if 0
	/// <summary>
	/// SI接頭辞テンプレートクラス
	/// 型を指定できる
	/// ※小数点のある型が望ましい
	/// </summary>
	/// <typeparam name="__ValueType"></typeparam>
	template<class __ValueType>
	class SIPrefixT : public NumeralT<__ValueType>
	{
	public:
		SIPrefixT(__ValueType init = 0) : NumeralT<__ValueType>(init)
		, Q(*this)
		, R(*this)
		, Y(*this)
		, Z(*this)
		, E(*this)
		, P(*this)
		, T(*this)
		, G(*this)
		, M(*this)
		, k(*this)
		, h(*this)
		, da(*this)
		, base(*this)
		, d(*this)
		, c(*this)
		, m(*this)
		, u(*this)
		, n(*this)
		, p(*this)
		, f(*this)
		, a(*this)
		, z(*this)
		, y(*this)
		, r(*this)
		, q(*this)
		{}

		///// <summary>
		///// SI接頭辞単位クラス
		///// </summary>
		///// <typeparam name="__ValueType"></typeparam>
		///// <typeparam name="__Exp"></typeparam>
		template<int __Exp = 0>
		class SIPrefixUnit : public BaseNumeral<__ValueType>
		{
		public:
			SIPrefixUnit(SIPrefixT<__ValueType>& r) :ref(r) {}

			//代入演算子
			inline BaseNumeral<__ValueType>& operator=(const BaseNumeral<__ValueType>& other)
			{
				return BaseNumeral<__ValueType>::operator=(other);
			}

		protected:
			/// <summary>
			/// Get関数
			/// プリミティブ型の値を返す
			/// </summary>
			/// <returns></returns>
			inline __ValueType GetValue()const override
			{
				return OriginalToSpecific(*ref);
			}

			/// <summary>
			/// Set関数
			/// プリミティブ型の値を設定する
			/// </summary>
			/// <param name="value"></param>
			/// <returns></returns>
			inline BaseNumeral<__ValueType>& SetValue(const __ValueType value)override
			{
				return ref(SpecificToOriginal(value));
			}

			/// <summary>
			/// 複製
			/// </summary>
			/// <returns></returns>
			BaseNumeral<__ValueType>& Clone(__ValueType init = 0)override
			{
				return *new SIPrefixT<__ValueType>(init);
			}

			//元の値を指定の単位へ
			__ValueType OriginalToSpecific(const __ValueType v)const
			{
				__ValueType value = v * std::pow(10, -__Exp);
				return value;
			}

			//指定の値を元の値へ
			__ValueType SpecificToOriginal(const __ValueType v)const
			{
				__ValueType value = v * std::pow(10, __Exp);
				return value;
			}

		private:
			SIPrefixT<__ValueType>& ref;
		};

		SIPrefixUnit< 30> Q;
		SIPrefixUnit< 27> R;
		SIPrefixUnit< 24> Y;
		SIPrefixUnit< 21> Z;
		SIPrefixUnit< 18> E;
		SIPrefixUnit< 15> P;
		SIPrefixUnit< 12> T;
		SIPrefixUnit<  9> G;
		SIPrefixUnit<  6> M;
		SIPrefixUnit<  3> k;
		SIPrefixUnit<  2> h;
		SIPrefixUnit<  1> da;
		SIPrefixUnit<  0> base;
		SIPrefixUnit<- 1> d;
		SIPrefixUnit<- 2> c;
		SIPrefixUnit<- 3> m;
		SIPrefixUnit<- 6> u;
		SIPrefixUnit<- 9> n;
		SIPrefixUnit<-12> p;
		SIPrefixUnit<-15> f;
		SIPrefixUnit<-18> a;
		SIPrefixUnit<-21> z;
		SIPrefixUnit<-24> y;
		SIPrefixUnit<-27> r;
		SIPrefixUnit<-30> q;

		//代入演算子
		inline BaseNumeral<__ValueType>& operator=(const SIPrefixT<__ValueType>& other)
		{
			return base = other;
		}

		inline BaseNumeral<__ValueType>& operator=(const BaseNumeral<__ValueType>& other)
		{
			return base = other;
		}
	};
	namespace {
		//static_assert(sizeof(SIPrefixT<int>) == sizeof(int) + sizeof(void*) * 4 * 25, "class SIPrefixT<int> Size Over Error");
	}


	/// <summary>
	/// SI接頭辞クラス
	/// </summary>
	using SIPrefix = SIPrefixT<long double>;
#endif

}

#endif // UNIT_OF_NUMBER_H
