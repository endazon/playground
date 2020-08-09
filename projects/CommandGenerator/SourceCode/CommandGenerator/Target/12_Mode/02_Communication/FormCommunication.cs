using System;
using System.Diagnostics;
using System.Drawing;
using System.Threading;
using System.Threading.Tasks;
using System.Windows.Forms;

using CommandGenerator.Class.Com;
using CommandGenerator.Class.Storage;

namespace CommandGenerator
{
	public partial class FormCommunication : FormEdit
	{
        private FormCommunicationDsplay FormCommunicationDsplay { get; set; } = null;

        //Socketクライアント
        private TcpClient tClient = null;

        private string host = "";

        private int port = 0;

        public FormCommunication()
		{
            Debug.WriteLine("FormCommunication" + " ThreadID:" + Thread.CurrentThread.ManagedThreadId);

            FormCommunicationDsplay = new FormCommunicationDsplay();
            InitializeComponent();
            InitCommunication();
        }

        public override void LocationUpdate(int x, int y)
        {
            base.LocationUpdate(x, y);
            FormCommunicationDsplay.LocationUpdate(x, y + Height);
        }

        public override void SizeUpdate(int width, int height)
        {
            base.SizeUpdate(width, height / 2);
            FormCommunicationDsplay.SizeUpdate(width, height / 2);
        }

        private void InitCommunication()
        {
            if (tClient != null) return;

            //Socketクライアント
            tClient = new TcpClient();

            //接続OKイベント
            tClient.OnConnected += new TcpClient.ConnectedEventHandler(tClient_OnConnected);
            //接続断イベント
            tClient.OnDisconnected += new TcpClient.DisconnectedEventHandler(tClient_OnDisconnected);
            //データ受信イベント
            tClient.OnReceiveData += new TcpClient.ReceiveEventHandler(tClient_OnReceiveData);
        }

        /** 接続断イベント **/
        private void tClient_OnDisconnected(object sender, EventArgs e)
        {
            Debug.WriteLine("tClient_OnDisconnected" + " ThreadID:" + Thread.CurrentThread.ManagedThreadId);
            if (this.InvokeRequired)
                this.Invoke(new DisconnectedDelegate(Disconnected), new object[] { sender, e });

            tClient = null;
        }
        private delegate void DisconnectedDelegate(object sender, EventArgs e);
        private void Disconnected(object sender, EventArgs e)
        {
            //接続断処理
            Debug.WriteLine("Disconnected" + " ThreadID:" + Thread.CurrentThread.ManagedThreadId);
        }


        /** 接続OKイベント **/
        private void tClient_OnConnected(EventArgs e)
        {
            //接続OK処理
            Debug.WriteLine("tClient_OnConnected" + " ThreadID:" + Thread.CurrentThread.ManagedThreadId);
        }

        /** データ受信イベント **/
        private void tClient_OnReceiveData(object sender, string e)
        {
            Debug.WriteLine("tClient_OnReceiveData" + " ThreadID:" + Thread.CurrentThread.ManagedThreadId);
            //別スレッドからくるのでInvokeを使用
            if (this.InvokeRequired)
                this.Invoke(new ReceiveDelegate(ReceiveData), new object[] { sender, e });
        }
        private delegate void ReceiveDelegate(object sender, string e);
        //データ受信処理
        private void ReceiveData(object sender, string e)
        {
            Debug.WriteLine("ReceiveData:" + e + " ThreadID:" + Thread.CurrentThread.ManagedThreadId);

            FormCommunicationDsplay.WriteLine("[" + DateTime.Now.ToString("yyyy/MM/dd HH:mm:ss")+ "] Rcve <<<<< " + e);
        }

        //接続処理
        // Connect to the remote endpoint.
        private void tConnect()
        {
            try
            {
                tClient.Connect(host, port);
            }
            catch (Exception ex)
            {
                MessageBox.Show(ex.Message);
            }
        }

        //送信処理
        private void tSend(string data)
        {
            try
            {
                tClient.Send(data);
                FormCommunicationDsplay.WriteLine("[" + DateTime.Now.ToString("yyyy/MM/dd HH:mm:ss") + "] Send >>>>> " + data);
            }
            catch (Exception ex)
            {
                MessageBox.Show(ex.Message);
            }
        }

        //切断処理
        private void tClose()
        {
            try
            {
                Debug.WriteLine("Form1_FormClosing" + " ThreadID:" + Thread.CurrentThread.ManagedThreadId);
                if (!tClient.IsClosed)
                    tClient.Close();

                tClient = null;
            }
            catch (Exception ex)
            {
                MessageBox.Show(ex.Message);
            }
        }

        #region Button
        public override void buttonRunning_Click(object sender, EventArgs e)
        {
            if (tClient == null)
            {
                InitCommunication();
                tConnect();
            }

            try
            {
                var cmd = (CommandCsvStorage.Item)GetSelectItem();
                tSend(cmd.Command);
            }
            catch (Exception ex)
            {
                Console.WriteLine(ex.Message);
                tClose();
            }
            finally
            {

            }
        }
        #endregion

        #region Window
        /** 接続処理 **/
        public override void FormEdit_Load(object sender, EventArgs e)
        {
            Debug.WriteLine("btn_Connect_Click" + " ThreadID:" + Thread.CurrentThread.ManagedThreadId);

            //TODO:要変更
            //接続先ホスト名
            host = new FormInputTextBox("ホスト名を入力して下さい。",
                                        "ホスト名の入力",
                                        "127.0.0.1"
                                        ).GetInputText();
            //TODO:要変更
            //接続先ポート
            port = Convert.ToInt32(new FormInputTextBox("ポート番号を入力して下さい。",
                                                        "ポート番号の入力",
                                                        "80"
                                                        ).GetInputText());
            tConnect();

            FormCommunicationDsplay.Show();
        }
        /** 切断処理 **/
        public override void FormEdit_FormClosed(object sender, FormClosedEventArgs e)
		{
            tClose();
            FormCommunicationDsplay.Close();
            FormCommunicationDsplay = null;
        }
		#endregion
	}
}
