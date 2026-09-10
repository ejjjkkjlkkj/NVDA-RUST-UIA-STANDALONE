import UIKit

@main
final class AppDelegate: UIResponder, UIApplicationDelegate {
    private var window: UIWindow?

    func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil
    ) -> Bool {
        let window = UIWindow(frame: UIScreen.main.bounds)
        let controller = UIViewController()
        controller.view.backgroundColor = .systemBackground

        let label = UILabel()
        label.translatesAutoresizingMaskIntoConstraints = false
        label.text = "Rust Screen Reader iOS accessibility probe"
        label.numberOfLines = 0
        label.textAlignment = .center
        label.isAccessibilityElement = true
        label.accessibilityLabel = "Rust Screen Reader iOS accessibility probe"
        label.accessibilityHint = "Validates the iOS accessibility client integration surface"
        label.accessibilityTraits = [.header]

        controller.view.addSubview(label)
        NSLayoutConstraint.activate([
            label.centerXAnchor.constraint(equalTo: controller.view.centerXAnchor),
            label.centerYAnchor.constraint(equalTo: controller.view.centerYAnchor),
            label.leadingAnchor.constraint(greaterThanOrEqualTo: controller.view.leadingAnchor, constant: 24),
            label.trailingAnchor.constraint(lessThanOrEqualTo: controller.view.trailingAnchor, constant: -24),
        ])

        window.rootViewController = controller
        window.makeKeyAndVisible()
        self.window = window

        UIAccessibility.post(notification: .screenChanged, argument: label)
        NSLog("IOS_ACCESSIBILITY_PROBE = PASS voiceOverRunning=%@", UIAccessibility.isVoiceOverRunning.description)
        return true
    }
}
