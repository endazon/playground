#ifndef UNIT_OF_NUMBER_H
#define UNIT_OF_NUMBER_H

#include <stdint.h>
#include <type_traits>
#include <cmath>
#include <cassert>

namespace UnitOfNumber {

#pragma region Numeral
	/// <summary>
	/// 数字実体クラス
	/// </summary>
	/// <typeparam name="__ValueType"></typeparam>
	template<class __ValueType>
	class NumericEntity
	{
	private:
		using __MySelfType = NumericEntity;
		__ValueType _Value;

	protected:
		template<class T>
		constexpr auto CAST(T&& v) { return static_cast<__ValueType>(v); }

		inline __ValueType GetValue() const noexcept
		{
			return _Value;
		}
		inline void SetValue(const __ValueType& v) & noexcept
		{
			_Value = v;
		}
		inline void SetValue(const __ValueType&& v) & noexcept
		{
			SetValue(v);
		}

	public:
		//**********************************************************
		//暗黙的に宣言される
		NumericEntity() noexcept = delete;
		//NumericEntity(const __MySelfType&) noexcept = delete;
		//NumericEntity(__MySelfType&&) noexcept = delete;
		constexpr ~NumericEntity() noexcept = default;
		//**********************************************************
		template<class T> constexpr NumericEntity(const T & other) noexcept : _Value(CAST(other))
		{}
		template<class T> constexpr NumericEntity(const T && other) noexcept : __MySelfType(other)
		{}

		//キャスト演算子(Cast)
		inline explicit operator __ValueType() const noexcept
		{
			return GetValue();
		}
	};

	/// <summary>
	/// 数字操作クラス
	/// </summary>
	/// <typeparam name="__InheritanceType"></typeparam>
	template<class __InheritanceType, class __ReturnType>
	class NumeralOperators : public __InheritanceType	{
		//※以下の関数はこのクラスを使用する際に必須
		//template<class T> constexpr auto CAST(T&& v);
		//inline __ValueType GetValue() const noexcept;
		//inline void SetValue(const __ValueType& v) & noexcept;
		//inline void SetValue(const __ValueType&& v) & noexcept;

	private:
		using __MySelfType = NumeralOperators;

	protected:
		template<class T>
		constexpr T mod(T lhs, T rhs) noexcept
		{
			if constexpr (std::is_integral<T>::value)
			{
				return lhs % rhs;
			}
			else if constexpr (std::is_floating_point<T>::value)
			{
				return std::fmod(lhs, rhs);
			}
		}

	public:
		//代入演算子(Assignment)
		//**********************************************************
		//暗黙的に宣言される
		//NumeralOperators& operator=(const NumeralOperators&) noexcept = delete;
		//NumeralOperators& operator=(NumeralOperators&&) & noexcept = delete;
		//**********************************************************
		template<class T> inline __MySelfType& operator=(T& rhs) noexcept
		{
			this->SetValue(this->CAST(rhs));
			return *this;
		}
		template<class T> inline __MySelfType& operator=(T&& rhs) & noexcept
		{
			this->SetValue(this->CAST(rhs));
			return *this;
		}

		//単項マイナス演算子と単項プラス演算子(Unary Negation/Plus)
		inline __ReturnType operator+() const { return __ReturnType(+this->GetValue()); }
		inline __ReturnType operator-() const { return __ReturnType(-this->GetValue()); }

		//算術演算子(Arithmetic)
		template<class T> inline __ReturnType operator+(T&& rhs) { return __ReturnType(this->GetValue() + this->CAST(rhs)); }
		template<class T> inline __ReturnType operator-(T&& rhs) { return __ReturnType(this->GetValue() - this->CAST(rhs)); }
		template<class T> inline __ReturnType operator*(T&& rhs) { return __ReturnType(this->GetValue() * this->CAST(rhs)); }
		template<class T> inline __ReturnType operator/(T&& rhs) { return __ReturnType(this->GetValue() / this->CAST(rhs)); }
		template<class T> inline __ReturnType operator%(T&& rhs) { return __ReturnType(mod(this->GetValue(), this->CAST(rhs))); }

		//複合代入演算子(Compound Assignment)
		template<class T> inline void operator+=(T&& rhs) { this->SetValue(this->GetValue() + this->CAST(rhs)); }
		template<class T> inline void operator-=(T&& rhs) { this->SetValue(this->GetValue() - this->CAST(rhs)); }
		template<class T> inline void operator*=(T&& rhs) { this->SetValue(this->GetValue() * this->CAST(rhs)); }
		template<class T> inline void operator/=(T&& rhs) { this->SetValue(this->GetValue() / this->CAST(rhs)); }
		template<class T> inline void operator%=(T&& rhs) { this->SetValue(mod(this->GetValue(), this->CAST(rhs))); }

		//後置インクリメント/デクリメント(Postfix Increment/Decrement)
		inline __ReturnType operator++(int) { auto z1 = this->GetValue(); this->SetValue(z1 + 1); return __ReturnType(z1); }
		inline __ReturnType operator--(int) { auto z1 = this->GetValue(); this->SetValue(z1 - 1); return __ReturnType(z1); }

		//前置インクリメント/デクリメント(Prefix Increment/Decremrnt)
		inline __MySelfType& operator++() { this->SetValue(this->GetValue() + 1); return *this; }
		inline __MySelfType& operator--() { this->SetValue(this->GetValue() - 1); return *this; }

		//論理否定演算子(Logical Not)
		inline bool operator!() const noexcept { return  this->GetValue() == 0; }

		//比較演算子(Compare)
		template<class T> inline bool operator==(T&& rhs) { return  this->GetValue() ==  this->CAST(rhs); }
		template<class T> inline bool operator!=(T&& rhs) { return  this->GetValue() !=  this->CAST(rhs); }
		template<class T> inline bool operator<=(T&& rhs) { return  this->GetValue() <=  this->CAST(rhs); }
		template<class T> inline bool operator< (T&& rhs) { return  this->GetValue() <   this->CAST(rhs); }
		template<class T> inline bool operator>=(T&& rhs) { return  this->GetValue() >=  this->CAST(rhs); }
		template<class T> inline bool operator> (T&& rhs) { return  this->GetValue() >   this->CAST(rhs); }

		//科学算術(Scientific Arithmetic)
		template<class T> inline __ReturnType pow(T&& rhs)	{ return __ReturnType(std::pow( this->GetValue(), this->CAST(rhs))); }
						  inline __ReturnType log()			{ return __ReturnType(std::log( this->GetValue())); }
		template<class T> inline __ReturnType log(T&& rhs)  { return __ReturnType(std::log( this->GetValue() / this->CAST(rhs))); }
						  inline __ReturnType abs()			{ return __ReturnType(std::abs( this->GetValue())); }
	};

	/// <summary>
	/// 基本数字クラス
	/// </summary>
	/// <typeparam name="__ValueType"></typeparam>
	template<class __ValueType>
	class BaseNumeral : public NumeralOperators<NumericEntity<__ValueType>, BaseNumeral<__ValueType>>
	{
	private:
		using __MySelfType = BaseNumeral;

	public:
		//**********************************************************
		//暗黙的に宣言される
		//BaseNumeral() noexcept = delete;
		//BaseNumeral(const __MySelfType&) noexcept = delete;
		//BaseNumeral(__MySelfType&&) noexcept = delete;
		constexpr ~BaseNumeral() noexcept = default;
		//**********************************************************
		constexpr BaseNumeral(const __ValueType & init) noexcept : NumeralOperators<NumericEntity<__ValueType>, BaseNumeral<__ValueType>>(init)
		{}
		constexpr BaseNumeral(const __ValueType && init = 0) noexcept : __MySelfType(init)
		{}
		template<class T> constexpr BaseNumeral(const T & other) noexcept : __MySelfType(this->CAST(other))
		{}
		template<class T> constexpr BaseNumeral(const T && other) noexcept : __MySelfType(other)
		{}
	};

	/// <summary>
	/// 数字クラス
	/// </summary>
	/// <typeparam name="__ValueType"></typeparam>
	template<class __ValueType>
	using Numeral = BaseNumeral<__ValueType>;
	namespace {
		using NumeralValueType = long double;
		static_assert(sizeof(Numeral<NumeralValueType>) == sizeof(NumeralValueType), "Numeral Size Error");
	}
#pragma endregion

/*
#pragma region FixedPointNumber
	/// <summary>
	/// 基本固定小数点数クラス
	/// </summary>
	/// <typeparam name="___MantissaType"></typeparam>
	/// <typeparam name="___ExponentType"></typeparam>
	template<class ___MantissaType, class ___ExponentType>
	class BaseFixedPointNumber
	{
		static_assert(
			std::is_arithmetic<___MantissaType>::value,
			"Mantissa can only be an arithmetic type."
			);
		static_assert(
			std::is_signed<___ExponentType>::value && std::is_integral<__ExponentType>::value,
			"Exponentiation can only be specified for integer and signed arithmetic types."
		);

	private:
		using __MantissaType = ___MantissaType;
		using __ExponentType = ___ExponentType;
		std::shared_ptr<__MantissaType> _pMantissa;
		std::shared_ptr<__ExponentType> _pExponent;

	protected:
		inline __MantissaType GetMantissa() const noexcept
		{
			return *_pMantissa;
		}
		inline void SetMantissa(const __MantissaType&& v) & noexcept
		{
			*_pMantissa = v;
		}
		inline __ExponentType GetExponent() const noexcept
		{
			return *_pExponent;
		}
		inline void SetExponent(const __ExponentType&& v) & noexcept
		{
			*_pExponent = v;
		}

		template<class T>
		constexpr auto M_CAST(T&& v) { return static_cast<__MantissaType>(v); }

		template<class T>
		constexpr auto E_CAST(T&& v) { return static_cast<__ExponentType>(v); }

		constexpr BaseFixedPointNumber(const BaseFixedPointNumber& other) noexcept
		{
			_pMantissa = other._pMantissa;
			_pExponent = other._pExponent;
		}

		template<class T>
		constexpr T mod(T lhs, T rhs) noexcept
		{
			if constexpr (std::is_integral<T>::value)
			{
				return lhs % rhs;
			}
			else if constexpr (std::is_floating_point<T>::value)
			{
				return std::fmod(lhs, rhs);
			}
		}

	public:
		//**********************************************************
		//暗黙的に宣言される
		//BaseFixedPointNumber() noexcept = delete;
		//BaseFixedPointNumber(const BaseFixedPointNumber&) = delete;
		BaseFixedPointNumber(BaseFixedPointNumber&&) noexcept = delete;
		~BaseFixedPointNumber() noexcept = default;
		//**********************************************************
		constexpr BaseFixedPointNumber(const __MantissaType& m_init, const __ExponentType& e_init) noexcept 
		: _pMantissa(std::make_shared<__MantissaType>(m_init))
		, _pExponent(std::make_shared<__ExponentType>(e_init))
		{}
		constexpr BaseFixedPointNumber(const __MantissaType && m_init, const __ExponentType && e_init) noexcept
		: BaseFixedPointNumber(m_init, e_init)
		{}

		//キャスト演算子(Cast)
		//inline explicit operator __ValueType() const noexcept
		//{
		//	return GetValue();
		//}

		//代入演算子(Assignment)
		//**********************************************************
		//暗黙的に宣言される
		BaseFixedPointNumber& operator=(const BaseFixedPointNumber&) = delete;
		//BaseFixedPointNumber& operator=(BaseFixedPointNumber&&) noexcept = delete;
		//**********************************************************
		inline BaseFixedPointNumber& operator=(BaseFixedPointNumber&& rhs) & noexcept
		{
			SetMantissa(rhs.SetMantissa());
			SetExponent(rhs.GetExponent());
			return *this;
		}

		////単項マイナス演算子と単項プラス演算子(Unary Negation/Plus)
		//inline BaseNumeral operator+() const { return BaseNumeral(+GetValue()); }
		//inline BaseNumeral operator-() const { return BaseNumeral(-GetValue()); }

		////算術演算子(Arithmetic)
		//template<class T> inline BaseNumeral operator+(T&& rhs) { return BaseNumeral(GetValue() + CAST(rhs)); }
		//template<class T> inline BaseNumeral operator-(T&& rhs) { return BaseNumeral(GetValue() - CAST(rhs)); }
		//template<class T> inline BaseNumeral operator*(T&& rhs) { return BaseNumeral(GetValue() * CAST(rhs)); }
		//template<class T> inline BaseNumeral operator/(T&& rhs) { return BaseNumeral(GetValue() / CAST(rhs)); }
		//template<class T> inline BaseNumeral operator%(T&& rhs) { return BaseNumeral(mod(GetValue(), CAST(rhs))); }

		////複合代入演算子(Compound Assignment)
		//template<class T> inline void operator+=(T&& rhs) { SetValue(GetValue() + rhs); }
		//template<class T> inline void operator-=(T&& rhs) { SetValue(GetValue() - rhs); }
		//template<class T> inline void operator*=(T&& rhs) { SetValue(GetValue() * rhs); }
		//template<class T> inline void operator/=(T&& rhs) { SetValue(GetValue() / rhs); }
		//template<class T> inline void operator%=(T&& rhs) { SetValue(GetValue() % rhs); }
		//template<>        inline void operator+=(BaseNumeral& rhs) { SetValue(GetValue() + rhs.GetValue()); }
		//template<>        inline void operator-=(BaseNumeral& rhs) { SetValue(GetValue() - rhs.GetValue()); }
		//template<>        inline void operator*=(BaseNumeral& rhs) { SetValue(GetValue() * rhs.GetValue()); }
		//template<>        inline void operator/=(BaseNumeral& rhs) { SetValue(GetValue() / rhs.GetValue()); }
		//template<>        inline void operator%=(BaseNumeral& rhs) { SetValue(mod(GetValue(), rhs.GetValue())); }

		////後置インクリメント/デクリメント(Postfix Increment/Decrement)
		//inline BaseNumeral operator++(int) { auto z1 = GetValue(); SetValue(z1 + 1); return BaseNumeral(z1); }
		//inline BaseNumeral operator--(int) { auto z1 = GetValue(); SetValue(z1 - 1); return BaseNumeral(z1); }

		////前置インクリメント/デクリメント(Prefix Increment/Decremrnt)
		//inline BaseNumeral& operator++() { SetValue(GetValue() + 1); return *this; }
		//inline BaseNumeral& operator--() { SetValue(GetValue() - 1); return *this; }

		////論理否定演算子(Logical Not)
		//inline bool operator!() const noexcept { return GetValue() != 0; }

		////比較演算子(Compare)
		//template<class T> inline bool operator==(T&& rhs) const { return GetValue() == rhs; }
		//template<class T> inline bool operator!=(T&& rhs) const { return GetValue() != rhs; }
		//template<class T> inline bool operator<=(T&& rhs) const { return GetValue() <= rhs; }
		//template<class T> inline bool operator< (T&& rhs) const { return GetValue() <  rhs; }
		//template<class T> inline bool operator> (T&& rhs) const { return GetValue() >  rhs; }
		//template<class T> inline bool operator>=(T&& rhs) const { return GetValue() >= rhs; }
		//template<>        inline bool operator==(BaseNumeral& rhs) const { return GetValue() == rhs.GetValue(); }
		//template<>        inline bool operator!=(BaseNumeral& rhs) const { return GetValue() != rhs.GetValue(); }
		//template<>        inline bool operator<=(BaseNumeral& rhs) const { return GetValue() <= rhs.GetValue(); }
		//template<>        inline bool operator< (BaseNumeral& rhs) const { return GetValue() <  rhs.GetValue(); }
		//template<>        inline bool operator> (BaseNumeral& rhs) const { return GetValue() >  rhs.GetValue(); }
		//template<>        inline bool operator>=(BaseNumeral& rhs) const { return GetValue() >= rhs.GetValue(); }

		////科学算術
		//template<class T> inline BaseNumeral pow(T&& rhs) { return BaseNumeral(std::pow(GetValue(), rhs)); }
		//template<>        inline BaseNumeral pow(BaseNumeral& rhs) { return BaseNumeral(std::pow(GetValue(), rhs.GetValue())); }
		//				  inline BaseNumeral log() { return BaseNumeral(std::log(GetValue())); }
		//template<class T> inline BaseNumeral log(T&& rhs) { return BaseNumeral(std::log(GetValue() / rhs)); }
		//template<>        inline BaseNumeral log(BaseNumeral& rhs) { return BaseNumeral(std::log(GetValue() / rhs.log())); }
		//				  inline BaseNumeral abs() { return BaseNumeral(std::abs(GetValue())); }
	};

	///// <summary>
	///// 数字クラス
	///// </summary>
	///// <typeparam name="__ValueType"></typeparam>
	//template<class __ValueType>
	//using Numeral = BaseNumeral<__ValueType>;
	//namespace {
	//	using NumeralValueType = long double;
	//	static_assert(sizeof(Numeral<NumeralValueType>) == sizeof(std::shared_ptr<NumeralValueType>), "Numeral Size Error");
	//}
#pragma endregion
*/

#pragma region SIPrefixUnit
	/// <summary>
	/// 基本SI接頭辞クラス
	/// 型を指定できる
	/// ※小数点のある型が望ましい
	/// </summary>
	/// <typeparam name="__ValueType"></typeparam>
	template<class __ValueType>
	class BaseSIPrefix : public NumeralOperators<NumericEntity<__ValueType>, BaseSIPrefix<__ValueType>>
	{
	private:
		using __MySelfType = BaseSIPrefix;

	protected:
		///// <summary>
		///// SI接頭辞単位実体クラス
		///// </summary>
		///// <typeparam name="__ValueType"></typeparam>
		///// <typeparam name="__Exp"></typeparam>
		template<int __Exp = 0>
		class SIPrefixUnitEntity
		{
		private:
			using __MySelfType = SIPrefixUnitEntity;
			BaseSIPrefix<__ValueType>& _Value;

		protected:
			template<class T>
			constexpr auto CAST(T&& v) { return static_cast<__ValueType>(v); }

			inline __ValueType GetValue() const noexcept
			{
				return static_cast<__ValueType>(_Value * std::pow(10, -__Exp));
			}
			inline void SetValue(const __ValueType& v) & noexcept
			{
				_Value = v * std::pow(10, __Exp);
			}
			inline void SetValue(const __ValueType&& v) & noexcept
			{
				SetValue(v);
			}

		public:
			//**********************************************************
			//暗黙的に宣言される
			SIPrefixUnitEntity() noexcept = delete;
			//SIPrefixUnitEntity(const __MySelfType&) noexcept = delete;
			//SIPrefixUnitEntity(__MySelfType&&) noexcept = delete;
			constexpr ~SIPrefixUnitEntity() noexcept = default;
			//**********************************************************
			template<class T> constexpr SIPrefixUnitEntity(T& other) noexcept : _Value(other)
			{}
			constexpr SIPrefixUnitEntity(__ValueType& other) noexcept : _Value(other)
			{}

			//キャスト演算子(Cast)
			inline explicit operator __ValueType() const noexcept
			{
				return GetValue();
			}
		};
		template<int __Exp = 0>
		using BaseSIPrefixUnit = NumeralOperators<SIPrefixUnitEntity<__Exp>, BaseSIPrefix<__ValueType>>;

	public:
		//**********************************************************
		//暗黙的に宣言される
		//BaseSIPrefix() noexcept = delete;
		//BaseSIPrefix(const __MySelfType&) noexcept = delete;
		//BaseSIPrefix(__MySelfType&&) noexcept = delete;
		//~BaseSIPrefix() noexcept = default;
		//**********************************************************
		constexpr BaseSIPrefix(const __ValueType& init) noexcept : NumeralOperators<NumericEntity<__ValueType>, BaseSIPrefix<__ValueType>>(init)
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
		constexpr BaseSIPrefix(const __ValueType&& init = 0) noexcept : __MySelfType(init)
		{}
		constexpr BaseSIPrefix(const __MySelfType& other) noexcept : __MySelfType(other.GetValue())
		{}
		constexpr BaseSIPrefix(const __MySelfType&& other) noexcept : __MySelfType(other)
		{}
		constexpr ~BaseSIPrefix() noexcept
		{}

		BaseSIPrefixUnit< 30> Q;
		BaseSIPrefixUnit< 27> R;
		BaseSIPrefixUnit< 24> Y;
		BaseSIPrefixUnit< 21> Z;
		BaseSIPrefixUnit< 18> E;
		BaseSIPrefixUnit< 15> P;
		BaseSIPrefixUnit< 12> T;
		BaseSIPrefixUnit<  9> G;
		BaseSIPrefixUnit<  6> M;
		BaseSIPrefixUnit<  3> k;
		BaseSIPrefixUnit<  2> h;
		BaseSIPrefixUnit<  1> da;
		BaseSIPrefixUnit<  0> base;
		BaseSIPrefixUnit<- 1> d;
		BaseSIPrefixUnit<- 2> c;
		BaseSIPrefixUnit<- 3> m;
		BaseSIPrefixUnit<- 6> u;
		BaseSIPrefixUnit<- 9> n;
		BaseSIPrefixUnit<-12> p;
		BaseSIPrefixUnit<-15> f;
		BaseSIPrefixUnit<-18> a;
		BaseSIPrefixUnit<-21> z;
		BaseSIPrefixUnit<-24> y;
		BaseSIPrefixUnit<-27> r;
		BaseSIPrefixUnit<-30> q;
	
		//代入演算子(Assignment)
		//**********************************************************
		//暗黙的に宣言される
		//BaseSIPrefix& operator=(const __MySelfType&) noexcept = delete;
		//BaseSIPrefix& operator=(__MySelfType&&) & noexcept = delete;
		//**********************************************************
		template<class T> inline __MySelfType& operator=(T& rhs) noexcept
		{
			this->SetValue(this->CAST(rhs));
			return *this;
		}
		template<class T> inline __MySelfType& operator=(T&& rhs) & noexcept
		{
			this->SetValue(this->CAST(rhs));
			return *this;
		}
	};

	/// <summary>
	/// SI接頭辞クラス
	/// </summary>
	template<class __ValueType>
	using SIPrefix = BaseSIPrefix<__ValueType>;
	namespace {
		using SIPrefixType = long double;
		static_assert(sizeof(SIPrefix<SIPrefixType>) == sizeof(SIPrefixType) + sizeof(SIPrefix<SIPrefixType>*) * 25, "SIPrefix Size Error");
	}
#pragma endregion


}

#endif // UNIT_OF_NUMBER_H
