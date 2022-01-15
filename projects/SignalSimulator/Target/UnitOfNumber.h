#ifndef UNIT_OF_NUMBER_H
#define UNIT_OF_NUMBER_H

#include <stdint.h>
#include <cassert>
#include <cmath>

namespace UnitOfNumber {

	/// <summary>
	/// 基本数字クラス
	/// </summary>
	/// <typeparam name="__ValueType"></typeparam>
	template<class __ValueType>
	class BaseNumeral
	{
	public:
		/// <summary>
		/// Get演算子
		/// プリミティブ型の値を返す
		/// </summary>
		/// <returns></returns>
		inline __ValueType operator*()const { return GetValue(); }

		/// <summary>
		/// Set演算子
		/// プリミティブ型の値を設定する
		/// </summary>
		/// <param name="value"></param>
		/// <returns></returns>
		inline BaseNumeral& operator()(const __ValueType value) { return SetValue(value); }

		//代入演算子
		inline BaseNumeral& operator=(const BaseNumeral& other) { return SetValue(other.GetValue()); }

		//単項演算子
		inline BaseNumeral& operator+() { return Clone(GetValue()); }
		inline BaseNumeral& operator-() { return Clone(-GetValue()); }

		//算術演算子
		inline BaseNumeral& operator+(BaseNumeral& other) { return Clone(GetValue() + other.GetValue()); }
		inline BaseNumeral& operator-(BaseNumeral& other) { return Clone(GetValue() - other.GetValue()); }
		inline BaseNumeral& operator*(BaseNumeral& other) { return Clone(GetValue() * other.GetValue()); }
		inline BaseNumeral& operator/(BaseNumeral& other) { return Clone(GetValue() / other.GetValue()); }
		inline BaseNumeral& operator%(BaseNumeral& other) { return Clone(GetValue() % other.GetValue()); }

		//科学算術
		inline BaseNumeral& pow(BaseNumeral& other) { return Clone(std::pow(GetValue(), other.GetValue())); }
		inline BaseNumeral& log(void) { return Clone(std::log(GetValue())); }
		inline BaseNumeral& log(BaseNumeral& other) { return (*this / other.log()).log(); }
		inline BaseNumeral& abs(void) { return Clone(std::abs(GetValue())); }

		//算術代入演算子
		inline void operator+=(BaseNumeral& other) { SetValue(GetValue() + other.GetValue()); }
		inline void operator-=(BaseNumeral& other) { SetValue(GetValue() - other.GetValue()); }
		inline void operator*=(BaseNumeral& other) { SetValue(GetValue() * other.GetValue()); }
		inline void operator/=(BaseNumeral& other) { SetValue(GetValue() / other.GetValue()); }
		inline void operator%=(BaseNumeral& other) { SetValue(GetValue() % other.GetValue()); }

		//比較演算子
		inline bool operator==(BaseNumeral& other) const { return GetValue() == other.GetValue(); }
		inline bool operator!=(BaseNumeral& other) const { return GetValue() != other.GetValue(); }
		inline bool operator<=(BaseNumeral& other) const { return GetValue() <= other.GetValue(); }
		inline bool operator< (BaseNumeral& other) const { return GetValue() <  other.GetValue(); }
		inline bool operator> (BaseNumeral& other) const { return GetValue() >  other.GetValue(); }
		inline bool operator>=(BaseNumeral& other) const { return GetValue() >= other.GetValue(); }

	protected:
		/// <summary>
		/// Get関数
		/// プリミティブ型の値を返す
		/// </summary>
		/// <returns></returns>
		virtual inline __ValueType GetValue()const = 0;

		/// <summary>
		/// Set関数
		/// プリミティブ型の値を設定する
		/// </summary>
		/// <param name="value"></param>
		/// <returns></returns>
		virtual inline BaseNumeral<__ValueType>& SetValue(const __ValueType value) = 0;

		/// <summary>
		/// 複製
		/// </summary>
		/// <returns></returns>
		virtual BaseNumeral<__ValueType>& Clone(__ValueType init = 0) = 0;
	};
	namespace {
		static_assert(sizeof(BaseNumeral<int>) == sizeof(void*) * 1, "BaseNumeral<int> Size Error");
	}

	/// <summary>
	/// 数字クラス
	/// </summary>
	/// <typeparam name="__ValueType"></typeparam>
	template<class __ValueType>
	class NumeralT : public BaseNumeral<__ValueType>
	{
	public:
		NumeralT(__ValueType init = 0) :value(init) {};

	protected:
		/// <summary>
		/// Get関数
		/// プリミティブ型の値を返す
		/// </summary>
		/// <returns></returns>
		inline __ValueType GetValue()const override
		{
			return value;
		}

		/// <summary>
		/// Set関数
		/// プリミティブ型の値を設定する
		/// </summary>
		/// <param name="value"></param>
		/// <returns></returns>
		inline BaseNumeral<__ValueType>& SetValue(const __ValueType value)override
		{
			this->value = value;
			return *this;
		}

		/// <summary>
		/// 複製
		/// </summary>
		/// <returns></returns>
		BaseNumeral<__ValueType>& Clone(__ValueType init = 0)override
		{
			return *new NumeralT<__ValueType>(init);
		}

	private:
		__ValueType value;
	};
	namespace {
		//static_assert(sizeof(NumeralT<int>) == sizeof(int) + sizeof(void*) * 1, "NumeralT<int> Size Error");
	}

	/// <summary>
	/// 数字クラス
	/// </summary>
	using Numeral = NumeralT<long double>;

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
}

#endif // UNIT_OF_NUMBER_H
