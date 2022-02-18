#pragma once

#include <mutex>
#include <thread>
#include "Utility.hpp"
#include "UnitOfNumber.hpp"
#include "GeneralPurposeTimer.hpp"

namespace Simulator {
	class Simulate : public UnitOfNumber::BaseSpecificNumeral<long double, UnitOfNumber::Numeral<long double>>
	{
	public:
		using __ValueType = long double;

	private:
		using __MySelfType = Simulate;
		using __InheritanceType = UnitOfNumber::BaseSpecificNumeral<__ValueType, UnitOfNumber::Numeral<__ValueType>>;

		inline static std::vector<__MySelfType*> signalList;

		void RegisteredSignalObjects(__MySelfType* pObj) noexcept
		{
			//DebuggingTimestampForRegistered(*pObj);

			signalList.push_back(pObj);

			signalInstanceUpdateFunction->Registered(*pObj);
		}
		void DeleteSignalObject(__MySelfType* pObj) noexcept
		{
			//DebuggingTimestampForDelete(*pObj);

			for (auto it = signalList.begin(); it != signalList.end();) {
				// 条件一致した要素を削除する
				if (*it == pObj) {
					// 削除された要素の次を指すイテレータが返される。
					it = signalList.erase(it);
					return;
				}
				// 要素削除をしない場合に、イテレータを進める
				++it;
			}
			signalInstanceUpdateFunction->Registered(*pObj);
		}

	protected:
		//オーバーライド
		void SetValue(const __ValueType& v) & noexcept override final
		{
			Changing();
			__InheritanceType::SetValue(v);
			Changed();

			//DebuggingTimestampForChanged();
		}

		//デバッグ用関数
		void DebuggingTimestamp(__MySelfType& rObj) noexcept
		{
			GeneralPurposeTimer::DateFormat::UTC date;
			std::cout << "【Name:" << rObj.Name() << "-Group:" << rObj.Group() << "-Comment:" << rObj.Comment() << "】";
			std::cout << date.format() << ":";
		}
		void DebuggingTimestampForRegistered(__MySelfType& rObj) noexcept
		{
			DebuggingTimestamp(rObj);
			std::cout << "Registered" << std::endl;
		}
		void DebuggingTimestampForDelete(__MySelfType& rObj) noexcept
		{
			DebuggingTimestamp(rObj);
			std::cout << "Delete" << std::endl;
		}

		void DebuggingTimestampForChanged() noexcept
		{
			DebuggingTimestamp(*this);
			std::cout << this->GetValue() << std::endl;
		}

	public:
		//using __InheritanceType::__InheritanceType; //継承元のコンストラクタは使わない
		//**********************************************************
		//暗黙的に宣言される
		//Simulate() noexcept = delete;
		//Simulate(const __MySelfType&) noexcept = delete;
		//Simulate(__MySelfType&&) noexcept = delete;
		//constexpr ~Simulate() noexcept = default;
		//**********************************************************
		inline Simulate(const __ValueType& init, const std::string name = "", const std::string group = "", const std::string comment = "") noexcept : __InheritanceType(init), _Name(name), _Group(group), _Comment(comment)
		{
			RegisteredSignalObjects(this);
		}
		inline Simulate(const __ValueType&& init = 0, const std::string name = "", const std::string group = "", const std::string comment = "") noexcept : __MySelfType(init, name, group, comment)
		{}
		template<class T> inline Simulate(const T& other, const std::string name = "", const std::string group = "", const std::string comment = "") noexcept : __MySelfType(this->CAST(other), name, group, comment)
		{}
		template<class T> inline Simulate(const T&& other, const std::string name = "", const std::string group = "", const std::string comment = "") noexcept : __MySelfType(other, name, group, comment)
		{}
		inline ~Simulate() noexcept
		{
			DeleteSignalObject(this);
		}

		//代入演算子(Assignment)
		//**********************************************************
		//暗黙的に宣言される
		//__MySelfType& operator=(const __MySelfType&) noexcept = delete;
		//__MySelfType& operator=(__MySelfType&&) & noexcept = delete;
		//**********************************************************
		template<class T> inline __MySelfType& operator=(T& rhs) noexcept
		{
			__InheritanceType::operator=(rhs);
			return *this;
		}

		template<class T> inline __MySelfType& operator=(T&& rhs) & noexcept
		{
			return operator=(rhs);
		}

		//BasicInformation
	private:
		const std::string _Name;
		const std::string _Group;
		const std::string _Comment;
		
	public:
		inline const std::string_view Name()    noexcept { return _Name;    }
		inline const std::string_view Group()   noexcept { return _Group;   }
		inline const std::string_view Comment() noexcept { return _Comment; }

		//SignalInstanceUpdateFunction
	public:
		class ISignalInstanceUpdateFunction
		{
		public:
			virtual void Registered(__MySelfType& rSignalInstance) = 0;
			virtual void Delete(__MySelfType& rSignalInstance) = 0;
		};
	private:
		class DummySignalInstanceUpdateFunction : public ISignalInstanceUpdateFunction
		{
			void Registered(__MySelfType& rSignalInstance) override{}
			void Delete(__MySelfType& rSignalInstance) override {}
		};
		inline static ISignalInstanceUpdateFunction* signalInstanceUpdateFunction = new DummySignalInstanceUpdateFunction();
	public:
		inline void RegisterSignalListAcquisitionFunction(ISignalInstanceUpdateFunction* func)
		{
			signalInstanceUpdateFunction = func;
			for (auto item : signalList)
			{
				signalInstanceUpdateFunction->Registered(*item);
			}
		}

		//
	protected: 
		//継承先で処理を定義する
		virtual void Changing() noexcept {}
		virtual void Changed() noexcept {}
	};

	class TimeSimulate : public Simulate
	{
	private:
		using __MySelfType = TimeSimulate;
		using __InheritanceType = Simulate;

		inline static std::mutex _Mutex;
		inline static std::thread* _Thread = nullptr;
		inline static std::vector<__MySelfType*> _TimeSimulatelList;

		void RegisteredSignalObjects(__MySelfType* pObj) noexcept
		{
			//DebuggingTimestampForRegistered(*pObj);

			{
				std::lock_guard<std::mutex> lock(_Mutex);
			
				_TimeSimulatelList.push_back(pObj);

				if (_Thread == nullptr) {
					_Thread = new std::thread(
						[this]() {
							do {
								UpdateProcess();
							} while (!_TimeSimulatelList.empty());
						}
					);

					_Thread->detach();
				}
			}
		}
		void DeleteSignalObject(__MySelfType* pObj) noexcept
		{
			//DebuggingTimestampForDelete(*pObj);

			{
				std::lock_guard<std::mutex> lock(_Mutex);

				for (auto it = _TimeSimulatelList.begin(); it != _TimeSimulatelList.end();) {
					// 条件一致した要素を削除する
					if (*it == pObj) {
						// 削除された要素の次を指すイテレータが返される。
						it = _TimeSimulatelList.erase(it);
						return;
					}
					// 要素削除をしない場合に、イテレータを進める
					++it;
				}
			}
		}

		inline static void UpdateProcess()
		{
			constexpr int BEGIN_COUNT = 0;
			constexpr int END_COUNT   = 10000 - 1;
			static    int count       = BEGIN_COUNT;
			static GeneralPurposeTimer::Measurement::ElapsedTimeDetection<GeneralPurposeTimer::Measurement::MediumPrecision> timer = 100;

			if (timer.isElapsed())
			{
				std::lock_guard<std::mutex> lock(_Mutex);

				for (auto i = count %     1; i < _TimeSimulatelList.size(); i +=     1) { _TimeSimulatelList[i]->UpdateIn100usCycle();  }
				for (auto i = count %     2; i < _TimeSimulatelList.size(); i +=     2) { _TimeSimulatelList[i]->UpdateIn200usCycle();  }
				for (auto i = count %     5; i < _TimeSimulatelList.size(); i +=     5) { _TimeSimulatelList[i]->UpdateIn500usCycle();  }
				for (auto i = count %    10; i < _TimeSimulatelList.size(); i +=    10) { _TimeSimulatelList[i]->UpdateIn1msCycle();    }
				for (auto i = count %    20; i < _TimeSimulatelList.size(); i +=    20) { _TimeSimulatelList[i]->UpdateIn2msCycle();    }
				for (auto i = count %    50; i < _TimeSimulatelList.size(); i +=    50) { _TimeSimulatelList[i]->UpdateIn5msCycle();    }
				for (auto i = count %   100; i < _TimeSimulatelList.size(); i +=   100) { _TimeSimulatelList[i]->UpdateIn10msCycle();   }
				for (auto i = count %   200; i < _TimeSimulatelList.size(); i +=   200) { _TimeSimulatelList[i]->UpdateIn20msCycle();   }
				for (auto i = count %   500; i < _TimeSimulatelList.size(); i +=   500) { _TimeSimulatelList[i]->UpdateIn50msCycle();   }
				for (auto i = count %  1000; i < _TimeSimulatelList.size(); i +=  1000) { _TimeSimulatelList[i]->UpdateIn100msCycle();  }
				for (auto i = count %  2000; i < _TimeSimulatelList.size(); i +=  2000) { _TimeSimulatelList[i]->UpdateIn200msCycle();  }
				for (auto i = count %  5000; i < _TimeSimulatelList.size(); i +=  5000) { _TimeSimulatelList[i]->UpdateIn500msCycle();  }
				for (auto i = count % 10000; i < _TimeSimulatelList.size(); i += 10000) { _TimeSimulatelList[i]->UpdateIn1000msCycle(); }

				count = count < END_COUNT ? count + 1 : BEGIN_COUNT;
			}
		}

	protected:
		virtual inline void UpdateIn100usCycle() noexcept {}
		virtual inline void UpdateIn200usCycle() noexcept {}
		virtual inline void UpdateIn500usCycle() noexcept {}
		virtual inline void UpdateIn1msCycle()   noexcept {}
		virtual inline void UpdateIn2msCycle()   noexcept {}
		virtual inline void UpdateIn5msCycle()   noexcept {}
		virtual inline void UpdateIn10msCycle()  noexcept {}
		virtual inline void UpdateIn20msCycle()  noexcept {}
		virtual inline void UpdateIn50msCycle()  noexcept {}
		virtual inline void UpdateIn100msCycle() noexcept {}
		virtual inline void UpdateIn200msCycle() noexcept {}
		virtual inline void UpdateIn500msCycle() noexcept {}
		virtual inline void UpdateIn1000msCycle()noexcept {}

	public:
		//using __InheritanceType::__InheritanceType; //継承元のコンストラクタは使わない
		//**********************************************************
		//暗黙的に宣言される
		//TimeSimulate() noexcept = delete;
		//TimeSimulate(const __MySelfType&) noexcept = delete;
		//TimeSimulate(__MySelfType&&) noexcept = delete;
		//constexpr ~TimeSimulate() noexcept = default;
		//**********************************************************
		inline TimeSimulate(const __ValueType& init, const std::string name = "", const std::string group = "", const std::string comment = "") noexcept : __InheritanceType(init, name, group, comment)
		{
			RegisteredSignalObjects(this);
		}
		inline TimeSimulate(const __ValueType&& init = 0, const std::string name = "", const std::string group = "", const std::string comment = "") noexcept : __MySelfType(init, name, group, comment)
		{}
		template<class T> inline TimeSimulate(const T& other, const std::string name = "", const std::string group = "", const std::string comment = "") noexcept : __MySelfType(this->CAST(other), name, group, comment)
		{}
		template<class T> inline TimeSimulate(const T&& other, const std::string name = "", const std::string group = "", const std::string comment = "") noexcept : __MySelfType(other, name, group, comment)
		{}
		inline ~TimeSimulate() noexcept
		{
			DeleteSignalObject(this);
		}
	};

	class Signal : public Simulate
	{
	private:
		using __MySelfType = Signal;
		using __InheritanceType = Simulate;

	protected:
		void Changing() noexcept override{}
		void Changed() noexcept override{}

	public:
		using __InheritanceType::__InheritanceType;

		//代入演算子(Assignment)
		//**********************************************************
		//暗黙的に宣言される
		//__MySelfType& operator=(const __MySelfType&) noexcept = delete;
		//__MySelfType& operator=(__MySelfType&&) & noexcept = delete;
		//**********************************************************
		template<class T> inline __MySelfType& operator=(T& rhs) noexcept
		{
			__InheritanceType::operator=(rhs);
			return *this;
		}

		template<class T> inline __MySelfType& operator=(T&& rhs) & noexcept
		{
			return operator=(rhs);
		}
	};

	class TimeSimulateTest : public TimeSimulate
	{
	private:
		using __MySelfType = TimeSimulateTest;
		using __InheritanceType = TimeSimulate;

	protected:
		void Changing() noexcept override{}
		void Changed() noexcept override{}

	public:
		using __InheritanceType::__InheritanceType; //継承元のコンストラクタは使わない

		//代入演算子(Assignment)
		//**********************************************************
		//暗黙的に宣言される
		//__MySelfType& operator=(const __MySelfType&) noexcept = delete;
		//__MySelfType& operator=(__MySelfType&&) & noexcept = delete;
		//**********************************************************
		template<class T> inline __MySelfType& operator=(T& rhs) noexcept
		{
			__InheritanceType::operator=(rhs);
			return *this;
		}

		template<class T> inline __MySelfType& operator=(T&& rhs) & noexcept
		{
			return operator=(rhs);
		}
	};
}