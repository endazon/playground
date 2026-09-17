#pragma once

#include <mutex>
#include <thread>
#include "UnitOfNumber.hpp"
#include "GeneralPurposeTimer.hpp"

namespace Simulator {
	class BaseSimulator : public UnitOfNumber::BaseSpecificNumeral<long double, UnitOfNumber::Numeral<long double>>
	{
	public:
		using __ValueType = long double;

	private:
		using __MySelfType = BaseSimulator;
		using __InheritanceType = UnitOfNumber::BaseSpecificNumeral<__ValueType, UnitOfNumber::Numeral<__ValueType>>;

		inline static std::vector<__MySelfType*> _SimulatorList;

		void RegisteredSignalObjects(__MySelfType* pObj) noexcept
		{
			//DebuggingTimestampForRegistered(*pObj);

			_SimulatorList.push_back(pObj);

			_SimulatorInstanceUpdateFunction->Registered(*pObj);
		}
		void DeleteSignalObject(__MySelfType* pObj) noexcept
		{
			//DebuggingTimestampForDelete(*pObj);

			for (auto it = _SimulatorList.begin(); it != _SimulatorList.end();) {
				// 条件一致した要素を削除する
				if (*it == pObj) {
					// 削除された要素の次を指すイテレータが返される。
					it = _SimulatorList.erase(it);
					_SimulatorInstanceUpdateFunction->Delete(*pObj);
					return;
				}
				// 要素削除をしない場合に、イテレータを進める
				++it;
			}
		}

	protected:
		//オーバーライド
		void SetValue(const __ValueType& v) & noexcept override
		{
			__InheritanceType::SetValue(v);
			_SimulatorInstanceUpdateFunction->ValueUpdate(*this);
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
		//BaseSimulator() noexcept = delete;
		//BaseSimulator(const __MySelfType&) noexcept = delete;
		//BaseSimulator(__MySelfType&&) noexcept = delete;
		//constexpr ~BaseSimulator() noexcept = default;
		//**********************************************************
		inline BaseSimulator(const __ValueType& init, const std::string name = "", const std::string group = "", const std::string comment = "") noexcept : __InheritanceType(init), _Name(name), _Group(group), _Comment(comment)
		{
			RegisteredSignalObjects(this);
		}
		inline BaseSimulator(const __ValueType&& init = 0, const std::string name = "", const std::string group = "", const std::string comment = "") noexcept : __MySelfType(init, name, group, comment)
		{}
		template<class T> inline BaseSimulator(const T& other, const std::string name = "", const std::string group = "", const std::string comment = "") noexcept : __MySelfType(this->CAST(other), name, group, comment)
		{}
		template<class T> inline BaseSimulator(const T&& other, const std::string name = "", const std::string group = "", const std::string comment = "") noexcept : __MySelfType(other, name, group, comment)
		{}
		inline ~BaseSimulator() noexcept
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
			virtual void ValueUpdate(__MySelfType& rSignalInstance) = 0;
		};
	private:
		class DummySignalInstanceUpdateFunction : public ISignalInstanceUpdateFunction
		{
		public:
			static DummySignalInstanceUpdateFunction* GetInstance() { static DummySignalInstanceUpdateFunction dummy; return &dummy; }
			void Registered(__MySelfType& rSignalInstance) override{}
			void Delete(__MySelfType& rSignalInstance) override {}
			void ValueUpdate(__MySelfType& rSignalInstance) override {}
		};
		inline static ISignalInstanceUpdateFunction* _SimulatorInstanceUpdateFunction = DummySignalInstanceUpdateFunction::GetInstance();
	public:
		inline static void RegisterSignalListAcquisitionFunction(ISignalInstanceUpdateFunction* func)
		{
			if (_SimulatorInstanceUpdateFunction == func) { return; }
			_SimulatorInstanceUpdateFunction = func;
			for (auto item : _SimulatorList)
			{
				_SimulatorInstanceUpdateFunction->Registered(*item);
			}
		}
		inline static void DeleteSignalListAcquisitionFunction()
		{
			_SimulatorInstanceUpdateFunction = DummySignalInstanceUpdateFunction::GetInstance();
		}
	};

	class BaseTimeSimulator : public BaseSimulator
	{
	private:
		using __MySelfType = BaseTimeSimulator;
		using __InheritanceType = BaseSimulator;

		inline static std::mutex _Mutex;
		inline static std::thread* _Thread = nullptr;
		inline static std::vector<__MySelfType*> _TimeSimulatorList;
		
		inline static void UpdateProcess()
		{
			constexpr int BEGIN_COUNT = 0;
			constexpr int END_COUNT   = 10000 - 1;
			static    int count       = BEGIN_COUNT;
			static GeneralPurposeTimer::Measurement::ElapsedTimeDetection<GeneralPurposeTimer::Measurement::MediumPrecision> timer = 100;

			if (timer.isElapsed())
			{
				std::lock_guard<std::mutex> lock(_Mutex);

				for (auto i = count %     1; i < _TimeSimulatorList.size(); i +=     1) { _TimeSimulatorList[i]->UpdateIn100usCycle();  }
				for (auto i = count %     2; i < _TimeSimulatorList.size(); i +=     2) { _TimeSimulatorList[i]->UpdateIn200usCycle();  }
				for (auto i = count %     5; i < _TimeSimulatorList.size(); i +=     5) { _TimeSimulatorList[i]->UpdateIn500usCycle();  }
				for (auto i = count %    10; i < _TimeSimulatorList.size(); i +=    10) { _TimeSimulatorList[i]->UpdateIn1msCycle();    }
				for (auto i = count %    20; i < _TimeSimulatorList.size(); i +=    20) { _TimeSimulatorList[i]->UpdateIn2msCycle();    }
				for (auto i = count %    50; i < _TimeSimulatorList.size(); i +=    50) { _TimeSimulatorList[i]->UpdateIn5msCycle();    }
				for (auto i = count %   100; i < _TimeSimulatorList.size(); i +=   100) { _TimeSimulatorList[i]->UpdateIn10msCycle();   }
				for (auto i = count %   200; i < _TimeSimulatorList.size(); i +=   200) { _TimeSimulatorList[i]->UpdateIn20msCycle();   }
				for (auto i = count %   500; i < _TimeSimulatorList.size(); i +=   500) { _TimeSimulatorList[i]->UpdateIn50msCycle();   }
				for (auto i = count %  1000; i < _TimeSimulatorList.size(); i +=  1000) { _TimeSimulatorList[i]->UpdateIn100msCycle();  }
				for (auto i = count %  2000; i < _TimeSimulatorList.size(); i +=  2000) { _TimeSimulatorList[i]->UpdateIn200msCycle();  }
				for (auto i = count %  5000; i < _TimeSimulatorList.size(); i +=  5000) { _TimeSimulatorList[i]->UpdateIn500msCycle();  }
				for (auto i = count % 10000; i < _TimeSimulatorList.size(); i += 10000) { _TimeSimulatorList[i]->UpdateIn1000msCycle(); }

				count = count < END_COUNT ? count + 1 : BEGIN_COUNT;
			}
		}

		void RegisteredSignalObjects(__MySelfType* pObj) noexcept
		{
			//DebuggingTimestampForRegistered(*pObj);

			{
				std::lock_guard<std::mutex> lock(_Mutex);
			
				_TimeSimulatorList.push_back(pObj);

				if (_Thread == nullptr) {
					_Thread = new std::thread(
						[this]() {
							do {
								UpdateProcess();
							} while (!_TimeSimulatorList.empty());
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

				for (auto it = _TimeSimulatorList.begin(); it != _TimeSimulatorList.end();) {
					// 条件一致した要素を削除する
					if (*it == pObj) {
						// 削除された要素の次を指すイテレータが返される。
						it = _TimeSimulatorList.erase(it);
						return;
					}
					// 要素削除をしない場合に、イテレータを進める
					++it;
				}
			}
		}

	public:
		//using __InheritanceType::__InheritanceType; //継承元のコンストラクタは使わない
		//**********************************************************
		//暗黙的に宣言される
		//BaseTimeSimulator() noexcept = delete;
		//BaseTimeSimulator(const __MySelfType&) noexcept = delete;
		//BaseTimeSimulator(__MySelfType&&) noexcept = delete;
		//constexpr ~BaseTimeSimulator() noexcept = default;
		//**********************************************************
		inline BaseTimeSimulator(const __ValueType& init, const std::string name = "", const std::string group = "", const std::string comment = "") noexcept : __InheritanceType(init, name, group, comment)
		{
			RegisteredSignalObjects(this);
		}
		inline BaseTimeSimulator(const __ValueType&& init = 0, const std::string name = "", const std::string group = "", const std::string comment = "") noexcept : __MySelfType(init, name, group, comment)
		{}
		template<class T> inline BaseTimeSimulator(const T& other, const std::string name = "", const std::string group = "", const std::string comment = "") noexcept : __MySelfType(this->CAST(other), name, group, comment)
		{}
		template<class T> inline BaseTimeSimulator(const T&& other, const std::string name = "", const std::string group = "", const std::string comment = "") noexcept : __MySelfType(other, name, group, comment)
		{}
		inline ~BaseTimeSimulator() noexcept
		{
			DeleteSignalObject(this);
		}

	protected:
		virtual inline void UpdateIn100usCycle() noexcept = 0;
		virtual inline void UpdateIn200usCycle() noexcept = 0;
		virtual inline void UpdateIn500usCycle() noexcept = 0;
		virtual inline void UpdateIn1msCycle()   noexcept = 0;
		virtual inline void UpdateIn2msCycle()   noexcept = 0;
		virtual inline void UpdateIn5msCycle()   noexcept = 0;
		virtual inline void UpdateIn10msCycle()  noexcept = 0;
		virtual inline void UpdateIn20msCycle()  noexcept = 0;
		virtual inline void UpdateIn50msCycle()  noexcept = 0;
		virtual inline void UpdateIn100msCycle() noexcept = 0;
		virtual inline void UpdateIn200msCycle() noexcept = 0;
		virtual inline void UpdateIn500msCycle() noexcept = 0;
		virtual inline void UpdateIn1000msCycle()noexcept = 0;
	};
}